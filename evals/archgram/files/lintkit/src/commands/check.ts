import { readFileSync, writeFileSync } from "node:fs";
import glob from "fast-glob";
import { loadRules } from "../plugins/registry.js";

export async function check() {
  const config = JSON.parse(readFileSync("lintkit.config.json", "utf8"));
  const levels = JSON.parse(readFileSync(".lintkit/rules.json", "utf8"));
  const rules = (await loadRules()).filter((r) => levels[r.name] !== "off");
  const results = glob.sync(config.include).flatMap((file) => {
    const text = readFileSync(file, "utf8");
    return rules.flatMap((r) => r.check(file, text).map((message) => ({ file, rule: r.name, message })));
  });
  writeFileSync(".lintkit/results.json", JSON.stringify(results, null, 2));
}
