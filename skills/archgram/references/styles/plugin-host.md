# Plugin host (microkernel)

- **Recognise:** a host that loads extensions by manifest or registry
  (`plugins/`, `extensions/`, a `plugin.json`, entry points declared in a
  package), with an API the extensions call back.
- **Parts:** the host, the extension API, the extensions, what each adds,
  and the files an extension installs into the host's project.
- **Questions:** what does the host own, and what does an extension add?
  What does installing one change?
- **Draw:** the host and its API as the core; extensions with the same
  relations as one node; what an extension writes into a project in a
  frame for that project. Where the extension's files are installed
  elsewhere, name the installed path and say where the source lives.
