import { readdirSync, readFileSync } from "node:fs";

export type Rule = { name: string; check(file: string, text: string): string[]; fix?(text: string): string };

export async function loadRules(): Promise<Rule[]> {
  const config = JSON.parse(readFileSync("lintkit.config.json", "utf8"));
  const local = readdirSync("plugins").map((file) => `../../plugins/${file}`);
  const modules = await Promise.all([...local, ...config.plugins].map((name) => import(name)));
  return modules.map((m) => m.rule as Rule);
}
