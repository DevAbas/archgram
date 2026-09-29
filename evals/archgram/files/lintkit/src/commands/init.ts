import { copyFileSync, mkdirSync } from "node:fs";

export async function init() {
  mkdirSync(".lintkit", { recursive: true });
  copyFileSync(new URL("../../templates/lintkit.config.json", import.meta.url), "lintkit.config.json");
  copyFileSync(new URL("../../templates/rules.json", import.meta.url), ".lintkit/rules.json");
}
