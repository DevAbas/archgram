// The launcher's choices, with Node's own test runner: `node --test lib/`.
'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { PLATFORMS, packageFor, binaryName, binary } = require('./platform.js');

test('each built platform has its package', () => {
  assert.equal(packageFor('darwin', 'arm64'), '@archgram/cli-darwin-arm64');
  assert.equal(packageFor('linux', 'x64'), '@archgram/cli-linux-x64');
  assert.equal(packageFor('win32', 'arm64'), '@archgram/cli-win32-arm64');
  assert.equal(PLATFORMS.length, 6);
});

test('a platform with no binary has no package', () => {
  assert.equal(packageFor('freebsd', 'x64'), null);
  assert.equal(packageFor('linux', 'ia32'), null);
});

test('Windows binaries end in .exe', () => {
  assert.equal(binaryName('win32'), 'archgram.exe');
  assert.equal(binaryName('linux'), 'archgram');
});

test('ARCHGRAM_BINARY overrides the package', () => {
  const found = binary({ ARCHGRAM_BINARY: '/opt/archgram' }, 'freebsd', 'x64', () => {
    throw new Error('not consulted');
  });
  assert.deepEqual(found, { file: '/opt/archgram' });
});

test('the binary sits in its package bin folder', () => {
  const manifest = path.join('/nm', '@archgram', 'cli-linux-x64', 'package.json');
  const found = binary({}, 'linux', 'x64', () => manifest);
  assert.equal(found.file, path.join('/nm', '@archgram', 'cli-linux-x64', 'bin', 'archgram'));
});

test('a missing package says why and how to reinstall', () => {
  const found = binary({}, 'darwin', 'arm64', () => {
    throw new Error('Cannot find module');
  });
  assert.match(found.error, /@archgram\/cli-darwin-arm64/);
  assert.match(found.error, /--omit=optional/);
  assert.match(found.error, /npm\/cli#4828/);
});

test('an unsupported platform lists the supported ones', () => {
  const found = binary({}, 'aix', 'ppc64', () => '');
  assert.match(found.error, /no binary for aix ppc64/);
  assert.match(found.error, /darwin-arm64/);
});
