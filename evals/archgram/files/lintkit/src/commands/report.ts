import { readFileSync, writeFileSync } from "node:fs";

export async function report() {
  const results: { file: string; rule: string; message: string }[] = JSON.parse(readFileSync(".lintkit/results.json", "utf8"));
  const rows = results.map((r) => `<tr><td>${r.file}</td><td>${r.rule}</td><td>${r.message}</td></tr>`).join("");
  writeFileSync(".lintkit/report.html", `<table>${rows}</table>`);
}
