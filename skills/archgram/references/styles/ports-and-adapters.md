# Hexagonal (ports and adapters)

- **Recognise:** a core with no framework imports (`domain/`, `core/`),
  interfaces it owns (`ports/`), and implementations outside it
  (`adapters/`, `infrastructure/`); also "clean" or "onion" architecture.
- **Parts:** driving adapters (HTTP, CLI, a queue consumer), the core,
  driven adapters (database, email, a payment API).
- **Questions:** what may the core depend on? What can be swapped?
- **Draw:** the core in the middle, driving adapters before it, driven
  adapters after it; adapters of one kind with the same relations are one
  node. The one idea is usually that every arrow points into the core's
  ports.
