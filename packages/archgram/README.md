# archgram

Architecture diagrams from a spec: archgram lays out the nodes, routes the
edges and draws one SVG, in light and dark, animated along the flows the
spec names.

```sh
npm install --save-dev archgram
npx archgram build diagram.json
```

The command is a native binary. npm installs the one for this machine
(macOS, Linux or Windows, on x64 or arm64) from an optional dependency,
`@archgram/cli-<os>-<cpu>`; no install script runs and nothing is
downloaded at install time. `archgram --help` lists the commands.

If npm was told to leave optional dependencies out (`--omit=optional`),
or the lockfile was written on another platform, the binary is missing:
delete `node_modules` and `package-lock.json` and run `npm install` again.
`ARCHGRAM_BINARY` points the command at another binary.
