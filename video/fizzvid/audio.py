# The video's sound: narration from Piper (text to speech), the game's own AdLib music recorded from DOSBox, and the mix.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import hashlib
import os
import shutil
import subprocess
import tempfile
import wave
from pathlib import Path

import numpy as np

from . import assets

RATE = 44100


def _decode(path):
    """Any audio file as mono float32 at RATE, through ffmpeg."""
    out = subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-f', 'f32le', '-ac', '1', '-ar', str(RATE), '-'],
                         check=True, capture_output=True).stdout
    return np.frombuffer(out, np.float32).copy()


def piper_command():
    exe = os.environ.get('PIPER') or shutil.which('piper')
    if not exe:
        raise FileNotFoundError('piper not found: see video/README.md (or render with --no-voice)')
    return exe


def speak(text, voice, cache):
    """Narration for one line as mono float32 at RATE, cached by the text and the voice."""
    key = hashlib.sha1(f'{voice}\n{text}'.encode()).hexdigest()[:16]
    wav = Path(cache) / 'voice' / f'{key}.wav'
    if not wav.exists():
        wav.parent.mkdir(parents=True, exist_ok=True)
        tmp = wav.with_suffix('.tmp.wav')
        subprocess.run([piper_command(), '-m', str(voice), '-f', str(tmp), '--sentence_silence', '0.15'], input=text.encode(),
                       check=True, capture_output=True)
        tmp.rename(wav)
    return _decode(wav)


def capture_song(name, seconds, cache, exe=None):
    """The game's song `name` played by FIZZ.EXE on DOSBox's OPL2 for `seconds`, as stereo float32 at RATE (cached).

    FIZZ.EXE's `soundtest song<n> secs<n> nofx` plays one song; DOSBox Staging writes its mixer to a file through SDL's disk
    audio driver, in real time."""
    out = Path(cache) / 'music' / f'{name}-{seconds}.f32'
    if out.exists():
        return np.fromfile(out, np.float32).reshape(-1, 2)
    root = assets.ROOT
    exe = Path(exe or root / 'm4/c/fizz.exe')
    dosbox = next((root / 'vendor/dosbox').glob('*/dosbox'), None)
    if not exe.exists() or dosbox is None:
        raise FileNotFoundError('needs m4/c/fizz.exe (m4/build.sh --no-run) and DOSBox Staging in vendor/ (tools/setup.sh)')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as d:
        d = Path(d)
        shutil.copy(exe, d / 'FIZZ.EXE')
        shutil.copy(root / 'vendor/cwsdpmi/bin/CWSDPMI.EXE', d / 'CWSDPMI.EXE')
        (d / 'cap.conf').write_text('[sdl]\nfullscreen = false\n[cpu]\ncore = dynamic\ncycles = max\n'
                                    '[sblaster]\nsbtype = sb2\noplmode = opl2\n[mixer]\nnosound = false\nrate = 44100\n')
        env = dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='disk', SDL_DISKAUDIOFILE=str(d / 'out.raw'))
        cmd = [str(dosbox), '--noprimaryconf', '--nolocalconf', '--conf', str(d / 'cap.conf'), '-c', f'mount c {d}', '-c', 'c:',
               '-c', f'FIZZ.EXE 0 0 soundtest adlib song{assets.song_index(name)} secs{seconds} nofx > OUT.TXT', '-c', 'exit']
        subprocess.run(cmd, env=env, timeout=seconds + 120, capture_output=True)
        raw = np.fromfile(d / 'out.raw', np.float32).reshape(-1, 2)
    # Start at the first note, and bring every song to the same loudness.
    loud = np.nonzero(np.abs(raw).max(axis=1) > 1e-4)[0]
    raw = raw[loud[0]:] if len(loud) else raw
    rms = float(np.sqrt(np.mean(raw ** 2))) or 1.0
    raw = (raw * (0.12 / rms)).clip(-1, 1).astype(np.float32)
    raw.tofile(out)
    return raw


def write_wav(path, stereo):
    pcm = (np.clip(stereo, -1, 1) * 32767).astype('<i2')
    with wave.open(str(path), 'wb') as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(pcm.tobytes())


def mix(total, voice_parts, music_parts, music_gain=0.55, duck=0.32):
    """The soundtrack: `voice_parts` [(start seconds, mono samples)] over `music_parts` [(start, end, stereo samples)], the music
    lowered under the voice and faded between songs."""
    n = int(total * RATE) + RATE
    out = np.zeros((n, 2), np.float32)
    voice = np.zeros(n, np.float32)
    for start, samples in voice_parts:
        i = int(start * RATE)
        seg = samples[:max(0, n - i)]
        voice[i:i + len(seg)] += seg
    # How loud the music is: full between lines, ducked while the voice speaks, with smooth changes.
    speaking = np.convolve((np.abs(voice) > 0.01).astype(np.float32), np.ones(RATE // 4, np.float32), 'same') > 0
    gain = np.where(speaking, duck, 1.0).astype(np.float32)
    k = RATE // 3
    gain = np.convolve(gain, np.ones(k, np.float32) / k, 'same')
    fade = RATE  # a second's crossfade between songs
    for start, end, samples in music_parts:
        i0, i1 = int(start * RATE), min(int(end * RATE), n)
        length = i1 - i0
        if length <= 0:
            continue
        reps = int(np.ceil(length / len(samples)))
        seg = np.tile(samples, (reps, 1))[:length].copy()
        ramp = np.ones(length, np.float32)
        f = min(fade, length // 2)
        ramp[:f] = np.linspace(0, 1, f)
        ramp[-f:] = np.linspace(1, 0, f)
        out[i0:i1] += seg * ramp[:, None] * music_gain
    out *= gain[:, None]
    out += voice[:, None] * 0.95
    # A soft limit for the odd peak, so nothing clips.
    over = np.abs(out) > 0.8
    out[over] = np.sign(out[over]) * (0.8 + 0.2 * np.tanh((np.abs(out[over]) - 0.8) / 0.2))
    return out[:int(total * RATE)]
