export const rule = {
  name: "max-lines",
  check: (_file: string, text: string) => (text.split("\n").length > 300 ? ["over 300 lines"] : []),
};
