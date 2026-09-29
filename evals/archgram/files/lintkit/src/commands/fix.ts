import { readFileSync, writeFileSync } from "node:fs";
import { loadRules } from "../plugins/registry.js";
import { check } from "./check.js";

export async function fix() {
  const results: { file: string; rule: string }[] = JSON.parse(readFileSync(".lintkit/results.json", "utf8"));
  const rules = await loadRules();
  for (const { file, rule } of results) {
    const fixer = rules.find((r) => r.name === rule)?.fix;
    if (fixer) writeFileSync(file, fixer(readFileSync(file, "utf8")));
  }
  await check();
}
