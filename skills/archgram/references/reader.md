# The reader

A diagram is judged in the first thirty seconds by someone who did not
build the system. Decide who that is and what they need before writing a
single node; everything else serves this.

## Contents

- Who reads it
- The reader contract
- Questions by kind of system
- Picking the shape
- The critique

## Who reads it

Take the reader from the context: the README's audience, the document the
diagram goes into, what the user said. Ask only when nothing tells you.

| Reader | What they are doing | What they expect |
|---|---|---|
| New contributor | Finding where to make a change | Names that exist in the repository; what calls what; where data lives; what is generated and must not be edited |
| Domain expert | Judging whether the architecture is sound | The mechanism: where a model is called, where checks happen, what runs at build time and what at run time |
| Reviewer of a task | Checking each requirement was met | Each requirement visible, and the one decision that makes the solution good shown rather than claimed |
| Stakeholder | Understanding what the system does | Few nodes, plain words, one flow from input to outcome |
| Operator | Running or deploying it | Services, stores, external systems, what runs where |

A README usually has two readers. The diagram serves the one who will
judge it; the prose serves the other.

## The reader contract

Write it in three to six lines and put it in the report:

```
Reader: <who>
They must be able to answer, from the diagram alone:
  1. <question>
  2. <question>
  3. <question>
The one idea the picture must make obvious: <the non-obvious property>
Left out on purpose: <what, and where it is explained instead>
```

The one idea is what tells this system apart from the obvious version of
it: "the app checks the model's citations", "code never reads the
palette", "every write goes through the ledger". If the diagram would look
the same for any system of its kind, it has not found the idea. A flow in
the spec is how the idea moves: make the contract's main path a flow.

A diagram that answers four questions well beats one that answers ten
badly.

## Questions by kind of system

Start from these and keep the three to five the reader cares most about.

- **Web app.** What happens between the user's action and the response?
  What runs in the browser, on the server, in a third-party service? Where
  is state kept, and who writes it? Where are the trust boundaries?
- **LLM, RAG or agent systems.** What does the model see, and never see?
  Which calls are model calls, and which deterministic code? Exact or
  semantic retrieval? What stops the model inventing facts, and where?
  What is prepared ahead (indexing) versus at question time?
- **Services and events.** Which services exist and which own which data?
  What is a call, what a message on a queue? What happens when one is down?
- **Data pipelines.** Where does data enter, change and land? Batch or
  streaming, on what schedule? Where is it validated, where does bad data
  go?
- **Build pipelines, design systems, code generation.** What is written by
  hand, what generated, what must never be edited? What does the build
  read and write, in what order? What enforces the rules: lint, hooks, CI?
- **CI/CD.** What triggers a build, what does it run, where does it
  deploy? Which environments, and where are the secrets?

## Picking the shape

| The reader's main question | In the spec |
|---|---|
| "What happens, in order?" | One flow along the main path, `direction: right` |
| "What is authored, built, generated, consumed?" | Frames for the stages, in flow order |
| "What runs before use, and what at run time?" | A frame for each phase, the stores between them |
| "Which part owns what?" | A frame for each owner, edges only for the calls that matter |
| "What checks what?" | A frame for the checks, each edge naming what it reads |

Two readers with different questions may need two diagrams: say so rather
than cramming both into one, and write two specs.

## The critique

Run it on the drawing before you report, and fix what it finds.

1. **Contract.** Can each question be answered from the picture alone?
2. **The one idea.** Is it visible without reading any note?
3. **Truth.** Is every node and edge backed by a file you read? Any name
   that does not exist in the code?
4. **Edge meaning.** Does every edge mean the same kind of thing (a call,
   data moving)? "Contains", "configures" and "references" are frames or
   notes, not edges.
5. **Level of detail.** Is an implementation detail shown while a
   load-bearing idea is missing? Is anything there only because it exists
   in the code?
6. **Missing steps.** Any hop the data takes that the picture skips: an
   embedding call, a build step, a check?
7. **Width.** GitHub shows a README image about 880 px wide. A drawing much
   wider than about 1,300 px shrinks its text below a comfortable size
   there: prefer `direction: down`, fewer nodes in a row, or two diagrams.
8. **Clutter.** A label on an edge that could be a node's note; a line that
   detours round the whole drawing; a frame that groups nothing a reader
   needs.
9. **Honest gaps.** What the diagram still does not show. Say it in the
   report; do not hide it.
