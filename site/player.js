// Starts the game in js-dos when Play is pressed, and wires the sound, fullscreen, restart and saves buttons.
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
  var downloadButton = document.getElementById('download');
  var uploadButton = document.getElementById('upload');
  var savesFile = document.getElementById('saves-file');
  var savesStatus = document.getElementById('saves-status');
  var player = null;
  var ci = null;
  var savesBusy = false;
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

  // The saves buttons need a running emulator, and a restart makes a new one.
  function showSaves() {
    downloadButton.disabled = uploadButton.disabled = !ci || savesBusy;
  }

  function say(message) {
    savesStatus.textContent = message;
  }

  // The names of the game's saves in the drive's root, from js-dos's file tree.
  function listSaves() {
    return ci.fsTree().then(function (tree) {
      return (tree.nodes || [])
        .filter(function (node) { return !node.nodes && FizzSaves.isSaveName(node.name); })
        .map(function (node) { return node.name; });
    });
  }

  function readSaves(names) {
    var files = [];
    return names
      .reduce(function (chain, name) {
        return chain.then(function () {
          return ci.fsReadFile(name).then(function (data) { files.push({ name: name, data: data }); });
        });
      }, Promise.resolve())
      .then(function () { return files; });
  }

  function download() {
    var running = ci;
    savesBusy = true;
    showSaves();
    listSaves()
      .then(readSaves)
      .then(function (files) {
        if (!files.length) {
          say('No saves yet: choose Save game in the pause menu, then try again.');
          return;
        }
        var link = document.createElement('a');
        link.href = URL.createObjectURL(FizzSaves.zip(files));
        link.download = 'lieutenant-fizz-saves.zip';
        document.body.appendChild(link);
        link.click();
        link.remove();
        setTimeout(function () { URL.revokeObjectURL(link.href); }, 10000);
        say('Downloaded ' + files.map(function (f) { return f.name; }).join(', ') + '.');
      })
      .catch(function (error) { say('Could not read the saves: ' + (error && error.message ? error.message : error)); })
      .then(function () {
        savesBusy = false;
        if (ci === running) showSaves();
      });
  }

  // A chosen file as save files: the files inside a zip, or the file itself.
  function filesOf(file) {
    return file.arrayBuffer().then(function (buffer) {
      var bytes = new Uint8Array(buffer);
      if (FizzSaves.isZip(bytes)) return FizzSaves.unzip(bytes);
      return [{ name: FizzSaves.baseName(file.name), data: bytes }];
    });
  }

  function upload(files) {
    var running = ci;
    var written = [];
    var skipped = [];
    savesBusy = true;
    showSaves();
    return Promise.all(Array.prototype.map.call(files, function (file) {
      return filesOf(file).catch(function (error) {
        skipped.push(file.name + ': ' + (error && error.message ? error.message : error));
        return [];
      });
    }))
      .then(function (lists) {
        return [].concat.apply([], lists).reduce(function (chain, entry) {
          return chain.then(function () {
            var why = FizzSaves.problem(entry.name, entry.data);
            if (why) {
              skipped.push(entry.name + ': ' + why);
              return null;
            }
            return ci.fsWriteFile(entry.name, entry.data).then(function () { written.push(entry.name); });
          });
        }, Promise.resolve());
      })
      .catch(function (error) { skipped.push('stopped: ' + (error && error.message ? error.message : error)); })
      .then(function () {
        var parts = [];
        if (written.length) parts.push('Put back ' + written.join(', ') + '. The game shows them the next time a save menu is opened.');
        if (skipped.length) parts.push('Left out: ' + skipped.join('; ') + '.');
        say(parts.join(' ') || 'Nothing to put back.');
        savesBusy = false;
        if (ci === running) showSaves();
      });
  }

  function launch() {
    host.hidden = false;
    ci = null;
    showSaves();
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
          showSaves();
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
    ci = null;
    showSaves();
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
  downloadButton.addEventListener('click', download);
  uploadButton.addEventListener('click', function () { savesFile.click(); });
  savesFile.addEventListener('change', function () {
    var chosen = Array.prototype.slice.call(savesFile.files);
    savesFile.value = '';
    if (chosen.length && ci) upload(chosen);
  });
  playButton.addEventListener('click', start);
  restartButton.addEventListener('click', restart);
  showMuted();
  if (params.has('autoplay')) start();
})();
