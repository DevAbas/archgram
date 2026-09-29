# Finding the architecture in the code

The reader contract says what the diagram must answer; this says how to
find the parts and lines that answer it, so the drawing is complete at the
level it chose, named as the code names things, and has no line the code
does not draw.

## Contents

- One level per diagram
- Start from the entry points
- Name a part after the file that decides
- Every line has a call site
- What is easy to miss

## One level per diagram

Pick one level, from the C4 model, and keep every node at it:

| Level | A node is | Suits |
|---|---|---|
| System context | The system itself, the people who use it, the systems it talks to | A stakeholder; the top of a README |
| Containers | Something that runs or is stored on its own: an app, a service, a CLI, a hook, a CI job, a database, a queue, a folder of generated files | A new contributor; most READMEs |
| Components | A module inside one container: a file or folder with one job | A reviewer of that container |

A diagram that mixes levels (three services and one helper function)
tells the reader the helper matters as much as a service. When the reader
needs two levels, draw two diagrams and say which is which.

## Start from the entry points

List every way the system is set going before you draw anything:

- commands: `package.json` scripts, a CLI's subcommands, a `Makefile`;
- routes and handlers: HTTP routes, message consumers, scheduled jobs;
- hooks: git hooks, an agent's hooks, a framework's lifecycle hooks;
- CI: each workflow and the jobs in it.

From each entry point the reader cares about, follow the code: what it
calls, what it reads, what it writes, what it starts. That walk is the
diagram at your chosen level. An entry point left out goes in the
contract's "left out on purpose", with where it is explained instead.

## Name a part after the file that decides

A node's label is the name the reader will search for. Find it by
following the imports from the entry point to the module where the work
is decided, not by a file name that looks right, a name in the docs, or
the first file you opened:

- a hook that imports `gates.mjs` and calls its `run()` is drawn going to
  `gates.mjs`, even when a `run-gates.mjs` sits beside it;
- a thin adapter (it only translates input and output) and the core it
  calls are two nodes, and the adapter's note says it only translates;
- where the docs and the code name a part differently, use the code's
  name and say so in the report.

## Every line has a call site

An edge is drawn only where the code makes it: a call, an import used at
run time, a read or a write of a store, a message sent. For each edge,
note the file and line that makes it (`src/api.ts:42`); the report lists
them. An edge you cannot point at is removed, however likely it seems.

"Configures", "contains" and "documents" are not edges: show them with a
frame or a note.

## What is easy to miss

Check each before drawing; draw the ones at your level that the reader's
questions touch, and list the rest as left out.

- **Modes of one entry point.** One script with several stages
  (`before-commit`, `lint`, `staleness`) is one node with each stage on
  the edges it takes, or one flow per stage; not only the stage you read
  first.
- **Enforcement.** Lint rules, hooks, CI checks and guards that stop a
  change: they are often the one idea the diagram must show.
- **Outputs.** Generated files, build artefacts, caches, logs: what is
  written, by whom, and whether it may be edited by hand.
- **Setup.** What must run once before anything works: a build, a
  migration, an index.
- **External systems.** Services, APIs and models the code calls: drawn
  as external, with their technology's logo.
- **Every caller of a core.** When several entry points share one core,
  each is a node with its edge into it.
