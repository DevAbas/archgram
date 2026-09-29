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
- Same relations, one node
- How much to draw
- What is easy to miss
- Styles of architecture

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

Every edge in one diagram means one kind of thing. When most edges move
data (reads and writes), one part starting another is a note or a flow's
order, not an edge beside them; when most start other parts, a store they
share is a note. "Configures", "contains" and "documents" are not edges:
show them with a frame or a note.

## Same relations, one node

Whatever the architecture, parts the reader need not tell apart can be
one node, and the drawing says so without knowing the style: two parts
are the same to the reader when the same nodes lead into them and they
lead to the same nodes. Tokens, a rules document and source code that
one command reads and another writes are one node, "design sources",
with their names in its note; a config file only a third command writes
stays its own. Merge only parts at the same level, and name the merged
node after what they are for, not after one of them.

Keep a part apart, even with the same relations, when a question in the
reader contract is about it, or when it is the one idea.

## How much to draw

About 10 nodes and 12 edges is what a reader takes in within thirty
seconds; start there, and treat it as a limit to argue with rather than a
rule. Over it:

1. merge parts with the same relations (above);
2. move what the contract does not ask about to "left out on purpose";
3. split into two diagrams, each with its own contract: one per entry
   point, per phase, or per level.

Completeness means nothing the reader's questions need is missing, not
that everything in the code is drawn. A loop (a report read by the
command that changes what the report reads) makes the drawing tall;
keep it when the loop is the idea, split it when it is not.

## What is easy to miss

Check each before drawing; draw the ones at your level that the reader's
questions touch, within how much to draw, and list the rest as left out.

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

## Styles of architecture

Most systems follow a known style, and each has its usual parts and
questions: `references/styles.md` lists them, with how to recognise each
in the code. Read the entry for the style you recognise; when none fits,
the rules above are enough.
