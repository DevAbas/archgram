// Which package holds the archgram binary for a machine, and what to say
// when it is missing. No side effects: the launcher (bin/archgram.js) runs
// the binary, and the tests call these directly.
'use strict';

const path = require('node:path');

/** The platforms a binary is built for, as `process.platform-process.arch`. */
const PLATFORMS = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64', 'win32-arm64', 'win32-x64'];

/** The package holding the binary for `platform` and `arch`, or null. */
function packageFor(platform, arch) {
  const key = `${platform}-${arch}`;
  return PLATFORMS.includes(key) ? `@archgram/cli-${key}` : null;
}

/** The binary's file name inside its package. */
function binaryName(platform) {
  return platform === 'win32' ? 'archgram.exe' : 'archgram';
}

/** Why no binary was found, and what to do about it. */
function missing(pkg, platform, arch) {
  if (pkg === null) {
    return (
      `archgram has no binary for ${platform} ${arch}. ` +
      `It is built for ${PLATFORMS.join(', ')}. ` +
      'Set ARCHGRAM_BINARY to an archgram binary built for this machine.'
    );
  }
  return (
    `archgram could not find ${pkg}, the package with its binary for this machine. ` +
    'npm leaves it out when optional dependencies are omitted (--omit=optional, --no-optional), ' +
    'or when package-lock.json was written on another platform (npm/cli#4828). ' +
    'Reinstall with optional dependencies: delete node_modules and package-lock.json, then run npm install.'
  );
}

/** The binary to run: ARCHGRAM_BINARY, else the platform package's. */
function binary(env, platform, arch, resolve) {
  if (env.ARCHGRAM_BINARY) {
    return { file: env.ARCHGRAM_BINARY };
  }
  const pkg = packageFor(platform, arch);
  if (pkg === null) {
    return { error: missing(pkg, platform, arch) };
  }
  let manifest;
  try {
    manifest = resolve(`${pkg}/package.json`);
  } catch {
    return { error: missing(pkg, platform, arch) };
  }
  return { file: path.join(path.dirname(manifest), 'bin', binaryName(platform)), pkg, manifest };
}

module.exports = { PLATFORMS, packageFor, binaryName, binary, missing };
