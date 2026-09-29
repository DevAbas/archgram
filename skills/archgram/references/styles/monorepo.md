# Libraries in a monorepo

- **Recognise:** workspaces (`packages/`, `apps/`, `crates/`, a
  `pnpm-workspace.yaml`, a Cargo workspace), each with its own manifest.
- **Parts:** the apps, the shared packages, and which depends on which.
- **Questions:** what may depend on what? What does each app pull in?
- **Draw:** one node per package, edges for dependencies only, with that
  meaning said in the description. Leaf utilities with the same
  dependents are one node.
