#!/usr/bin/env -S npx tsx
import { init } from "./commands/init.js";
import { check } from "./commands/check.js";
import { fix } from "./commands/fix.js";
import { report } from "./commands/report.js";

const commands = { init, check, fix, report };
const [name = "check"] = process.argv.slice(2);
await commands[name as keyof typeof commands]();
