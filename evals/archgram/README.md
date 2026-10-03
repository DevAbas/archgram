# Evaluations of the archgram skill

Eight cases for `skills/archgram`, in the format of Anthropic's
[skill-creator](https://github.com/anthropics/skills/tree/main/skills/skill-creator),
the one the Agent Skills format's own guide to evaluating skills uses:
`evals.json` holds each prompt and what a good run produces, and `files/`
the small projects the prompts run in.

| Case | What it checks |
|---|---|
| `draws-a-project` | It draws a project from its code, every part backed by a file and every edge by a line, no wider than 1,300 px, in the colours of its CSS, without starting a browser |
| `asks-when-unclear` | It asks when the README and the code disagree, rather than drawing what does not exist |
| `updates-an-existing-diagram` | It changes an existing spec and says what changed |
| `not-for-other-work` | It stays out of a request that is not a diagram |
| `draws-a-pipeline` | It recognises a data pipeline and follows one day's rows from their sources to the warehouse, with where bad rows go |
| `draws-ports-and-adapters` | It recognises ports and adapters and shows the rules depending on nothing outside their ports |
| `merges-parts-with-the-same-relations` | It merges parts no question tells apart and keeps a crowded system within about 10 nodes and 12 edges |
| `repairs-what-lost-its-code` | It starts from what `archgram check` says has lost its code, removes only that, and keeps every source the check found |

Every case that draws expects a source on each node a file backs and on
each edge, so it needs an archgram that reads them (0.7 or later;
docs/features/sources.md).

Each case runs with the skill and without it, as skill-creator does. The
runs call a model and cost money, so they run on request, not in CI; the
skill runs `npx archgram`, so the version under test must be on npm, and
the runs need network access to the npm registry.
