# Samples: a refused step, and how a card's border is drawn

Hand-made samples that chose the design in [DESIGN.md](../../../DESIGN.md)
(Components: Signal, Refusal), kept so the other options can be compared
later. They are not
archgram's output: each is `gates-retry.archgram.yaml` drawn by archgram
0.4.0, with the new animation added by hand on top.

The story is the same in every sample: a commit is refused at `checks/`,
the refusal goes back to the person, and the fixed commit then passes.

| File | What differs from the default |
|---|---|
| `spark.{light,dark}.svg` | Nothing: the default |
| `drain.{light,dark}.svg` | The border drains out through the card's leaving arrow |
| `ring.{light,dark}.svg` | The border sweeps once around from the arrowhead |
| `afterglow.{light,dark}.svg` | The border fades back to the card's own edge while the card is lit |
| `pending.{light,dark}.svg` | The refused card waits with a slow dashed border |
| `glow.{light,dark}.svg` | Signals, the ✕, the refusal and the border heads glow. The design chosen after it keeps a faint glow on a signal's line only |
