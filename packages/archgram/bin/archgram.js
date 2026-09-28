#!/usr/bin/env node
// archgram on npm (ARCHITECTURE.md, Distribution): finds the package with
// this platform's native binary, one of the optionalDependencies npm chose
// by its `os` and `cpu`, and runs it with this command's arguments. No
// dependencies, no install script.
'use strict';

const { spawnSync } = require('node:child_process');
const { binary } = require('../lib/platform.js');

function main() {
  const found = binary(process.env, process.platform, process.arch, require.resolve);
  if (found.error) {
    console.error(found.error);
    process.exit(1);
  }
  // The two packages come from one release; a mismatch means a broken install.
  if (found.manifest) {
    const own = require('../package.json').version;
    const theirs = require(found.manifest).version;
    if (own !== theirs) {
      console.error(
        `archgram ${own} found ${found.pkg} ${theirs}; they must be the same version. Reinstall archgram.`,
      );
      process.exit(1);
    }
  }
  const run = spawnSync(found.file, process.argv.slice(2), { stdio: 'inherit', windowsHide: true });
  if (run.error) {
    console.error(`archgram could not run ${found.file}: ${run.error.message}`);
    process.exit(1);
  }
  if (run.signal) {
    process.kill(process.pid, run.signal);
    return;
  }
  process.exit(run.status ?? 1);
}

main();
