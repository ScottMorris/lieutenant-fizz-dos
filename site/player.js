// Starts the game in js-dos when Play is pressed, and wires the sound and fullscreen buttons.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

(function () {
  var params = new URLSearchParams(window.location.search);
  var stage = document.getElementById('stage');
  var host = document.getElementById('dos');
  var cover = document.getElementById('cover');
  var playButton = document.getElementById('play');
  var muteButton = document.getElementById('mute');
  var fullButton = document.getElementById('full');
  var ci = null;
  var muted = params.has('mute');

  function showMuted() {
    muteButton.textContent = muted ? 'Sound off' : 'Sound on';
    muteButton.setAttribute('aria-pressed', String(muted));
  }

  function applyMute() {
    if (!ci) return;
    if (muted) ci.mute();
    else ci.unmute();
  }

  function start() {
    playButton.disabled = true;
    playButton.textContent = 'Loading…';
    host.hidden = false;
    window.Dos(host, {
      url: 'game.jsdos',
      pathPrefix: 'vendor/js-dos/emulators/',
      autoStart: true,
      kiosk: true,
      noCloud: true,
      onEvent: function (event, instance) {
        if (event === 'ci-ready') {
          ci = instance;
          applyMute();
        }
        if (event === 'emu-ready' || event === 'bnd-play') {
          cover.hidden = true;
        }
      },
    });
  }

  muteButton.addEventListener('click', function () {
    muted = !muted;
    showMuted();
    applyMute();
  });
  fullButton.addEventListener('click', function () {
    if (stage.requestFullscreen) stage.requestFullscreen();
  });
  playButton.addEventListener('click', start);
  showMuted();
  if (params.has('autoplay')) start();
})();
