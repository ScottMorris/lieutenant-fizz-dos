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
  var restartButton = document.getElementById('restart');
  var player = null;
  var ci = null;
  var muted = params.has('mute');
  // ?opl=default or ?opl=fast loads the same game with another OPL emulator, to compare the sound; anything else is the usual bundle.
  var opl = params.get('opl');
  var bundle = opl === 'default' || opl === 'fast' ? 'game-' + opl + '.jsdos' : 'game.jsdos';

  function showMuted() {
    muteButton.textContent = muted ? 'Sound off' : 'Sound on';
    muteButton.setAttribute('aria-pressed', String(muted));
  }

  function applyMute() {
    if (!ci) return;
    if (muted) ci.mute();
    else ci.unmute();
  }

  function launch() {
    host.hidden = false;
    ci = null;
    player = window.Dos(host, {
      url: bundle,
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
    restartButton.disabled = false;
  }

  function start() {
    playButton.disabled = true;
    playButton.textContent = 'Loading…';
    launch();
  }

  // Leaving the game and typing `exit` at the DOS prompt stops the emulator; this starts a fresh one from the start-up screen.
  function restart() {
    restartButton.disabled = true;
    var stopped = player && player.stop ? Promise.resolve(player.stop()).catch(function () {}) : Promise.resolve();
    stopped.then(function () {
      host.replaceChildren();
      launch();
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
  restartButton.addEventListener('click', restart);
  showMuted();
  if (params.has('autoplay')) start();
})();
