# Evaluations of the archgram skill

Four cases for `skills/archgram`, in the format of Anthropic's
[skill-creator](https://github.com/anthropics/skills/tree/main/skills/skill-creator):
`evals.json` holds each prompt and what a good run produces, and `files/`
the small projects the prompts run in.

| Case | What it checks |
|---|---|
| `draws-a-project` | It draws a project from its code, every part backed by a file, in the colours of its CSS |
| `asks-when-unclear` | It asks when the README and the code disagree, rather than drawing what does not exist |
| `updates-an-existing-diagram` | It changes an existing spec and says what changed |
| `not-for-other-work` | It stays out of a request that is not a diagram |

Each case runs with the skill and without it, as skill-creator does. The
runs call a model and cost money, so they run on request, not in CI; the
skill runs `npx archgram`, so the version under test must be on npm, and
the runs need network access to the npm registry.
