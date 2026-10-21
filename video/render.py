#!/usr/bin/env python3
# Renders the explainer video: footage from the game's own renderer, narration, the game's AdLib music, captions.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: video/render.py [--out FILE] [--scenes id,id] [--still SECONDS] [--scale 0.5] [--no-voice] [--no-music] [--jobs N]
#
# See video/README.md for what it needs. Everything slow is cached under out/video/: the footage, each narration line, each song.

import argparse
import json
import multiprocessing
import multiprocessing.pool
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from fizzvid import assets, audio, draw, scenes  # noqa: E402

ROOT = assets.ROOT
OUT = ROOT / 'out/video'
FPS = 30
LINE_GAP = 0.35


class Timed:
    """A scene placed on the timeline."""

    def __init__(self, scene, start, at, ends, dur, song_start):
        self.scene, self.start, self.at, self.ends, self.dur = scene, start, at, ends, dur
        self.song, self.lead, self.song_start = scene.song, scene.lead, song_start

    def song_t(self, t):
        """Seconds into this scene's song at scene time t (the song carries on across scenes that share it)."""
        return self.start + t - self.song_start


def footage(folder):
    """Records the clips with the bench (FOOTAGE=<dir>) unless they are there already."""
    if (folder / 'title').exists():
        return
    bench = ROOT / 'm5/bench'
    subprocess.run(['cargo', 'build', '--release', '--target', 'i586-unknown-linux-gnu'], cwd=bench, check=True)
    exe = bench / 'target/i586-unknown-linux-gnu/release/render-bench'
    folder.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(exe)], env=dict(os.environ, FOOTAGE=str(folder)), check=True)


def plan(chosen, voice, cache):
    """Times every scene from its narration; returns the timed scenes, the voice parts and the total length."""
    timed, voice_parts, t = [], [], 0.0
    song_start, last_song = 0.0, None
    for sc in chosen:
        at, ends, cursor = [], [], sc.lead
        for caption, spoken in sc.lines:
            if voice:
                samples = audio.speak(spoken, voice, cache)
                length = len(samples) / audio.RATE
                voice_parts.append((t + cursor, samples))
            else:
                length = max(1.5, len(spoken.split()) / 2.6)
            at.append(cursor)
            ends.append(cursor + length)
            cursor += length + LINE_GAP
        dur = max(sc.min_dur, cursor - LINE_GAP + sc.tail)
        if sc.song != last_song:
            song_start, last_song = t, sc.song
        timed.append(Timed(sc, t, at, ends, dur, song_start))
        t += dur
    return timed, voice_parts, t


def music_parts(timed, cache):
    """The songs under the scenes: consecutive scenes with the same song share one recording."""
    groups = []
    for ts in timed:
        if groups and groups[-1][0] == ts.song:
            groups[-1][2] = ts.start + ts.dur
        else:
            groups.append([ts.song, ts.start, ts.start + ts.dur])
    needed = {}
    for song, a, b in groups:
        needed[song] = max(needed.get(song, 0), int(b - a) + 4)
    with multiprocessing.pool.ThreadPool(4) as pool:
        recorded = dict(zip(needed, pool.starmap(audio.capture_song, [(s, n, cache) for s, n in needed.items()])))
    # Each song overlaps its neighbours by half a second either side, for the crossfade.
    return [(max(0.0, a - 0.5), b + 0.5, recorded[song]) for song, a, b in groups]


TIMED = []
SCALE = 1.0


def frame_at(t):
    """The picture at time t: the scene's own drawing and the caption of the line being spoken."""
    for ts in TIMED:
        if t < ts.start + ts.dur or ts is TIMED[-1]:
            break
    st = t - ts.start
    img = draw.Image.new('RGB', (draw.W, draw.H), draw.BG)
    ts.scene.draw(img, st, ts)
    for i, (caption, _) in enumerate(ts.scene.lines):
        until = ts.ends[i] + LINE_GAP if i + 1 == len(ts.at) else ts.at[i + 1] - 0.1
        if ts.at[i] - 0.1 <= st < until:
            draw.caption(img, caption, draw.ramp(st, ts.at[i] - 0.1, 0.15))
    # A short fade to black between scenes.
    edge = min(st, ts.dur - st)
    if edge < 0.25:
        k = draw.clamp(edge / 0.25)
        img = draw.Image.blend(draw.Image.new('RGB', img.size, (0, 0, 0)), img, 0.35 + 0.65 * k)
    if SCALE != 1.0:
        img = img.resize((int(draw.W * SCALE) // 2 * 2, int(draw.H * SCALE) // 2 * 2), draw.Image.BILINEAR)
    return img


def render_one(i):
    return frame_at(i / FPS).tobytes()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', default=str(OUT / 'explainer.mp4'))
    ap.add_argument('--scenes', help='comma-separated scene ids to render (default all)')
    ap.add_argument('--still', type=float, help='write one PNG at this time instead of a video')
    ap.add_argument('--scale', type=float, default=1.0, help='0.5 renders a quick half-size preview')
    ap.add_argument('--voice', default=os.environ.get('FIZZ_VOICE', str(OUT / 'voice/en-us-lessac-medium.onnx')))
    ap.add_argument('--no-voice', action='store_true', help='no narration (scenes timed from the word count)')
    ap.add_argument('--no-music', action='store_true')
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 2)
    ap.add_argument('--footage', default=str(OUT / 'footage'))
    ap.add_argument('--cache', default=str(OUT / 'cache'))
    args = ap.parse_args()

    global TIMED, SCALE
    SCALE = args.scale
    folder = Path(args.footage)
    footage(folder)
    for name in ['title', 'map', 'day', 'caves', 'night', 'theatre', 'pause', 'preview']:
        scenes.CLIPS[name] = assets.Clip(folder / name)
    chosen = scenes.SCENES
    if args.scenes:
        ids = args.scenes.split(',')
        chosen = [s for s in scenes.SCENES if s.id in ids]
    voice = None if args.no_voice else args.voice
    if voice and not Path(voice).exists():
        sys.exit(f'no voice at {voice}: run video/fetch-voice.sh, or render with --no-voice')
    TIMED, voice_parts, total = plan(chosen, voice, args.cache)
    print(f'{len(TIMED)} scenes, {total:.1f} s', file=sys.stderr)
    for ts in TIMED:
        print(f'  {ts.scene.id:<8} {ts.start:6.1f} s  {ts.dur:5.1f} s', file=sys.stderr)

    if args.still is not None:
        out = Path(args.out).with_suffix('.png') if args.out.endswith('.mp4') else Path(args.out)
        frame_at(args.still).save(out)
        print(out)
        return

    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    wav = Path(args.cache) / 'soundtrack.wav'
    wav.parent.mkdir(parents=True, exist_ok=True)
    music = [] if args.no_music else music_parts(TIMED, args.cache)
    audio.write_wav(wav, audio.mix(total, voice_parts, music))
    (Path(args.out).with_suffix('.json')).write_text(json.dumps(
        [{'id': ts.scene.id, 'start': round(ts.start, 2), 'length': round(ts.dur, 2)} for ts in TIMED], indent=1))

    frames = int(total * FPS)
    w, h = int(draw.W * SCALE) // 2 * 2, int(draw.H * SCALE) // 2 * 2
    enc = subprocess.Popen(['ffmpeg', '-y', '-v', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{w}x{h}', '-r', str(FPS),
                            '-i', '-', '-i', str(wav), '-c:v', 'libx264', '-preset', 'medium', '-crf', '20', '-pix_fmt', 'yuv420p',
                            '-c:a', 'aac', '-b:a', '192k', '-shortest', '-movflags', '+faststart', args.out], stdin=subprocess.PIPE)
    with multiprocessing.get_context('fork').Pool(args.jobs) as pool:
        for n, data in enumerate(pool.imap(render_one, range(frames), chunksize=4)):
            enc.stdin.write(data)
            if n % (FPS * 10) == 0:
                print(f'  frame {n} / {frames}', file=sys.stderr)
    enc.stdin.close()
    enc.wait()
    print(args.out)


if __name__ == '__main__':
    main()
