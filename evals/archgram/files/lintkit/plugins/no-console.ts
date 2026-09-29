export const rule = {
  name: "no-console",
  check: (_file: string, text: string) => (text.includes("console.log") ? ["console.log left in"] : []),
  fix: (text: string) => text.replaceAll(/^\s*console\.log\(.*\);?\n/gm, ""),
};
