# The explainer video

A six-minute video about Lieutenant Fizz for DOS: the game, the DOS era it is made for (Commander Keen, the 486, VGA, AdLib), how the port works and how it was made fast. Everything in it is drawn by code from the game's own material, so it can be changed and rendered again:

- **Footage** comes from the game's real renderer and shell, recorded by the native bench (`FOOTAGE=<dir>` in `m5/bench`), and is shown with the DOS palette, pixel for pixel. The caves and level 9 are played by the walk, from a small family, that survives longest there; the day level and the map use the scripted walk.
- **Music** is the game's own AdLib score: `FIZZ.EXE … soundtest song<n> secs<n> nofx` plays one song in DOSBox Staging, which records its mixer through SDL's disk audio driver.
- **The pixel font, sprites, palette and the OPL2 channel meters** are read from `m2/dos-rt/src/ui_data.rs`, `m3/assets/sprites.bin`, `m4/c/main.c`'s palette rule and the IMF data in `m4/assets/sound.bin`.
- **Narration** is Piper text to speech; **captions** show each line as it is spoken.

## Files

| File | What it holds |
|---|---|
| `fizzvid/scenes.py` | The 15 scenes: each one's narration (the caption, and the spoken form where the voice needs help), its song and its drawing. Edit the words here. `PRONOUNCE` fixes words the voice gets wrong ("DOS" is said "doss"). |
| `fizzvid/draw.py` | Drawing helpers: fonts, the game's pixel font, the CRT monitor for footage, panels, easing, captions. |
| `fizzvid/assets.py` | The palette, footage clips, font, sprites and the OPL2 channel activity of a song. |
| `fizzvid/audio.py` | Narration (cached by its text), song recording (cached) and the mix, with the music lowered under the voice. |
| `render.py` | Times the scenes from the narration, records the songs, mixes the sound and renders the frames into ffmpeg. |
| `fetch-voice.sh` | Installs Piper in a virtual environment and downloads the voice, into `out/video/`. |

## Rendering

It needs what `tools/setup.sh` sets up (the bench, `m4/c/fizz.exe`, DOSBox Staging in `vendor/`), plus `ffmpeg`, Python 3 with Pillow and NumPy, and the Inter and DejaVu fonts.

```bash
m4/build.sh --no-run                                   # FIZZ.EXE, which plays the songs
video/fetch-voice.sh                                   # Piper and the voice, into out/video/
PIPER=out/video/venv/bin/piper video/render.py         # out/video/explainer.mp4, about five minutes on four cores
```

- `--scenes keen,era` renders only those scenes; `--still 12.5` writes one PNG at that time; `--scale 0.5` renders a quick half-size preview; `--no-voice` times the scenes from their word count instead; `--no-music` leaves the music out.
- Footage, narration and songs are cached under `out/video/` (footage in `footage/`, the rest in `cache/`). Delete `out/video/footage` after a change to the renderer, the game or the bench's clips to record them again; a changed line of narration is spoken again on its own. Songs are recorded in real time, so the first render takes a minute or two longer.
- `out/video/explainer.json` lists where each scene starts.

## Licences

The video's code is the repository's (Apache-2.0 OR MIT). The default voice, Piper's `en-us-lessac-medium`, is trained on the Lessac Blizzard 2013 data, whose licence allows non-commercial use only; `en-us-ryan-medium` is CC BY-NC-SA 4.0. Choose another voice (`--voice` or `FIZZ_VOICE`) before using the video commercially. Commander Keen is a trademark of id Software; the video names it as history and as the series' inspiration, and Lieutenant Fizz is not affiliated with id Software or Apogee.
