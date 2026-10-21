#!/bin/bash
# Installs Piper (text to speech) in a virtual environment and downloads the narration voice, into out/video/.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: video/fetch-voice.sh [voice]      (default en-us-lessac-medium; others: en-us-ryan-medium, en-us-lessac-low)
#
# The voices come from Piper's v0.0.2 release on GitHub. Check a voice's licence (its MODEL_CARD) before publishing a video made
# with it: the Lessac and Ryan voices are for non-commercial use.
set -euo pipefail
cd "$(dirname "$0")/.."
VOICE="${1:-en-us-lessac-medium}"
DIR=out/video/voice
mkdir -p "$DIR"
if [ ! -x out/video/venv/bin/piper ]; then
  python3 -m venv out/video/venv
  out/video/venv/bin/pip install -q piper-tts
fi
if [ ! -f "$DIR/$VOICE.onnx" ]; then
  curl -sSfL -o "$DIR/voice-$VOICE.tar.gz" "https://github.com/rhasspy/piper/releases/download/v0.0.2/voice-$VOICE.tar.gz"
  tar -xzf "$DIR/voice-$VOICE.tar.gz" -C "$DIR"
  rm "$DIR/voice-$VOICE.tar.gz"
fi
ls -l "$DIR/$VOICE.onnx"
echo "Render with: PIPER=out/video/venv/bin/piper video/render.py"
