// Save files for the page: which names are the game's, whether a file is a save the game would read, and a small zip writer and
// reader so a player can take all their saves as one file and put them back.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

(function () {
  // The game's saves are AUTO.SAV, QUICK.SAV and SLOT1.SAV to SLOT4.SAV in the drive's root (m4/c/save.c); FIZZ.CFG, the options, is not a save.
  var NAME = /^(AUTO|QUICK|SLOT[1-4])\.SAV$/;
  // m2/dos-rt/src/save.rs reads a file only if it is 48 bytes long and starts with "FZSV" and a version byte of 1.
  var LENGTH = 48;
  var VERSION = 1;
  // The most a zip entry may expand to; a save is 48 bytes, so anything near this is not one.
  var LIMIT = 4096;

  var crcTable = null;
  function crc32(bytes) {
    if (!crcTable) {
      crcTable = new Uint32Array(256);
      for (var n = 0; n < 256; n++) {
        var c = n;
        for (var k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
        crcTable[n] = c >>> 0;
      }
    }
    var crc = 0xffffffff;
    for (var i = 0; i < bytes.length; i++) crc = crcTable[(crc ^ bytes[i]) & 255] ^ (crc >>> 8);
    return (crc ^ 0xffffffff) >>> 0;
  }

  // The name a file goes by in the game's drive: the last part of its path, in capitals.
  function baseName(path) {
    return String(path).split(/[\\/]/).pop().toUpperCase();
  }

  function isSaveName(name) {
    return NAME.test(name);
  }

  // Why a save would not be read by the game, or '' if it would be.
  function problem(name, bytes) {
    if (!isSaveName(name)) return 'not one of the game’s save names (AUTO.SAV, QUICK.SAV, SLOT1.SAV to SLOT4.SAV)';
    if (bytes.length !== LENGTH) return 'the wrong size (' + bytes.length + ' bytes, not ' + LENGTH + ')';
    if (bytes[0] !== 0x46 || bytes[1] !== 0x5a || bytes[2] !== 0x53 || bytes[3] !== 0x56) return 'not a Lieutenant Fizz save';
    if (bytes[4] !== VERSION) return 'from a different version of the game';
    return '';
  }

  // The time and date fields of a zip entry, in MS-DOS form.
  function dosStamp(date) {
    var time = (date.getHours() << 11) | (date.getMinutes() << 5) | (date.getSeconds() >> 1);
    var day = ((Math.max(date.getFullYear(), 1980) - 1980) << 9) | ((date.getMonth() + 1) << 5) | date.getDate();
    return [time, day];
  }

  // A zip of `files` ([{ name, data: Uint8Array }]), stored without compression: a save is 48 bytes.
  function zip(files, date) {
    var stamp = dosStamp(date || new Date());
    var encoder = new TextEncoder();
    var locals = [];
    var centrals = [];
    var offset = 0;
    files.forEach(function (f) {
      var name = encoder.encode(f.name);
      var crc = crc32(f.data);
      var local = new DataView(new ArrayBuffer(30));
      local.setUint32(0, 0x04034b50, true);
      local.setUint16(4, 20, true);
      local.setUint16(10, stamp[0], true);
      local.setUint16(12, stamp[1], true);
      local.setUint32(14, crc, true);
      local.setUint32(18, f.data.length, true);
      local.setUint32(22, f.data.length, true);
      local.setUint16(26, name.length, true);
      var central = new DataView(new ArrayBuffer(46));
      central.setUint32(0, 0x02014b50, true);
      central.setUint16(4, 20, true);
      central.setUint16(6, 20, true);
      central.setUint16(12, stamp[0], true);
      central.setUint16(14, stamp[1], true);
      central.setUint32(16, crc, true);
      central.setUint32(20, f.data.length, true);
      central.setUint32(24, f.data.length, true);
      central.setUint16(28, name.length, true);
      central.setUint32(42, offset, true);
      locals.push(new Uint8Array(local.buffer), name, f.data);
      centrals.push(new Uint8Array(central.buffer), name);
      offset += 30 + name.length + f.data.length;
    });
    var size = centrals.reduce(function (n, part) { return n + part.length; }, 0);
    var end = new DataView(new ArrayBuffer(22));
    end.setUint32(0, 0x06054b50, true);
    end.setUint16(8, files.length, true);
    end.setUint16(10, files.length, true);
    end.setUint32(12, size, true);
    end.setUint32(16, offset, true);
    return new Blob(locals.concat(centrals, [new Uint8Array(end.buffer)]), { type: 'application/zip' });
  }

  function isZip(bytes) {
    return bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 3 && bytes[3] === 4;
  }

  function inflate(bytes) {
    var stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
    return new Response(stream).arrayBuffer().then(function (buffer) { return new Uint8Array(buffer); });
  }

  // The files in a zip, [{ name, data }] with each name reduced to its last part in capitals. Entries stored or deflated are read;
  // entries that are folders or too big to be a save are left out.
  function unzip(bytes) {
    var view = new DataView(bytes.buffer, bytes.byteOffset, bytes.length);
    var decoder = new TextDecoder();
    var end = -1;
    for (var i = bytes.length - 22; i >= Math.max(0, bytes.length - 22 - 65535); i--) {
      if (view.getUint32(i, true) === 0x06054b50) { end = i; break; }
    }
    if (end < 0) return Promise.reject(new Error('not a zip file'));
    var count = view.getUint16(end + 10, true);
    var at = view.getUint32(end + 16, true);
    var jobs = [];
    for (var n = 0; n < count; n++) {
      if (at + 46 > bytes.length || view.getUint32(at, true) !== 0x02014b50) return Promise.reject(new Error('damaged zip file'));
      var method = view.getUint16(at + 10, true);
      var packed = view.getUint32(at + 20, true);
      var size = view.getUint32(at + 24, true);
      var nameLength = view.getUint16(at + 28, true);
      var extraLength = view.getUint16(at + 30, true);
      var commentLength = view.getUint16(at + 32, true);
      var local = view.getUint32(at + 42, true);
      var name = decoder.decode(bytes.subarray(at + 46, at + 46 + nameLength));
      at += 46 + nameLength + extraLength + commentLength;
      if (name.slice(-1) === '/' || size > LIMIT || packed > LIMIT) continue;
      if (local + 30 > bytes.length || view.getUint32(local, true) !== 0x04034b50) return Promise.reject(new Error('damaged zip file'));
      var start = local + 30 + view.getUint16(local + 26, true) + view.getUint16(local + 28, true);
      var body = bytes.subarray(start, start + packed);
      jobs.push(entry(baseName(name), method, body));
    }
    return Promise.all(jobs);
  }

  function entry(name, method, body) {
    var data = method === 0 ? Promise.resolve(body) : method === 8 ? inflate(body) : Promise.reject(new Error('a compression method this page cannot read'));
    return data.then(function (bytes) { return { name: name, data: bytes }; });
  }

  window.FizzSaves = { baseName: baseName, isSaveName: isSaveName, problem: problem, zip: zip, unzip: unzip, isZip: isZip };
})();
