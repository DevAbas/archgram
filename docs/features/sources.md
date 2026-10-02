# Feature: the code behind a diagram

| Field   | Value      |
|---------|------------|
| Version | 0.1        |
| Date    | 2026-10-02 |
| Status  | Draft      |
| Release | 0.7        |
| Issue   | [#46](https://github.com/byAbas/archgram/issues/46) |

What this feature does and why, in more detail than `docs/PRD.md`, which
points here. The field it adds is in `docs/SPEC.md` (Sources); how
archgram checks it is in `ARCHITECTURE.md` (Parse and validate).

## Problem

A diagram kept as text beside the code is cheap to change, but nothing
says when it must change (PRD §1). The `archgram` skill already finds the
file behind each part and the line behind each edge (PRD §6.6), but it
only lists them in its report: the spec keeps none of it, so nothing can
tell later that a part or a line has lost its code. With agents changing
a project many times a day, the diagram is wrong before anyone looks.

## Who it serves

| User | What they get |
|---|---|
| Whoever keeps the diagram, a person or an agent | Told what lost its code, at the spec's line, whenever they check or draw |
| The reviewer of a pull request | The `archgram check` CI already runs fails when a change removes code a diagram points at |
| The skill | What it claims about the code is checked as it draws, so a part or a line it cannot back is caught before the drawing |

The reader of the diagram never sees a source. They get a drawing that
stays true.

## Requirements

- A node and an edge may each name the code behind them, `source`: a
  path, or a list of paths for a node that stands for several parts. A
  path names a file or a folder, from the spec's folder.
- A source may carry, after `#`, a few words copied from the line that
  makes it, such as `../../src/worker.rs#links.insert(`. The words are
  looked for as written, so a source holds through edits elsewhere in the
  file and fails only when that code itself changes or goes. A line
  number would fail at every edit above it, and still say nothing about
  the call.
- Sources are never drawn: the SVG is the same with or without them.
- The check runs where archgram already runs, with nothing to turn on:
  - `archgram check` reports each source the code lacks as a problem, at
    its line and column, and fails;
  - `archgram build` still draws, and warns of each one;
  - the skill writes a source for every part and edge, so its
    `archgram check` holds them to the code as it draws; on a later call,
    what the check names is what the skill updates.
- How to write a source is in the format (`docs/SPEC.md`, Sources), which
  `archgram spec` prints, not in the skill. The skill is installed apart
  from archgram and may be newer or older than the archgram it runs; from
  the format, it writes sources exactly when that archgram reads them,
  with no version check in the skill and no change to it for this field.
- It reads files only: no git, no network, no clock, so CI can run it on
  every pull request and the same files always give the same answer. It
  says whether a path and its words are there, never what a file holds.

## Limit

The check finds what the code lost, not what it gained: a new part or a
new call is in no spec, so nothing points at it. Finding those is the
skill's work when it runs again.

## Not in this

- Redrawing by itself. The check says what lost its code; the skill, or a
  person, changes the spec.
- Checking what the code does: that the line still writes to the table
  the edge points at.

## Success criteria

- In each spec the skill writes, every node and every edge carries a
  source (100 %), and `archgram check` confirms each one when the drawing
  is made.
- On the skill's evaluation projects, after a change that deletes a part
  or the call behind a line, `archgram check` names each one affected,
  and after a change that keeps them, names none
  (`evals/archgram`, `repairs-what-lost-its-code`).

## Changelog

| Version | Date       | Change |
|---------|------------|--------|
| 0.1     | 2026-10-02 | First draft, from #46: a source on a node and an edge, a path with the words of its line after `#`; checked by `check` (fails) and `build` (warns), never drawn; the skill writes them and starts an update from what the check names; the limit, and the criteria. |
