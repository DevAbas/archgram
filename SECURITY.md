# Security policy

## Supported versions

Only the latest release gets security fixes.

| Version | Supported |
|---|---|
| 0.6.x | yes |
| below 0.6 | no |

## Reporting a vulnerability

Please do not report a vulnerability in a public issue.

Report it privately through GitHub: open the repository's **Security** tab
and choose **Report a vulnerability**. Include:

- what the problem is and what an attacker could do with it;
- the version of archgram, and how you installed it (npm or from source);
- the smallest spec, theme file or command that shows it.

You will get an answer as soon as possible. Once a fix is released, the
advisory is published with credit to you, unless you prefer otherwise.

## What archgram does

archgram reads the files it is given (a spec, and with `--theme-file` a
theme file and the design token files it names), and the sources a spec
names, and writes the diagram. It runs no scripts, makes no network
requests and downloads nothing, at install time or at run time. Its npm
packages have no dependencies and no install scripts, and are published
from GitHub Actions with provenance.

## What archgram reads

A spec, a theme and the sources a spec names may come from someone else,
such as a pull request that CI checks. So archgram reads every file the
same way:

- only a regular file, whose type is known before it is opened, so a
  FIFO or a device is never opened;
- at most 4 MiB of it;
- never through a symbolic link that may have come with it: a spec or
  theme file that is a link, or is reached through one under the folder
  archgram runs in, is refused, and so is a source or a theme's file on
  the way to which there is one;
- a theme's token files only under the theme file's folder, and a
  spec's sources only under the project's folder (the nearest above the
  spec that holds `.git`, else the folder archgram runs in, never a
  disk's root), judged by the path's text before the disk is looked at,
  so a spec cannot learn whether a file outside the project exists;
- never `.git`, nor a file that commonly holds secrets rather than code
  (docs/SPEC.md, Sources, lists them), as a source or a theme's file;
- each file once per run, keeping only whether it holds a source's
  words, and at most 1000 sources per spec, so a spec cannot make a run
  slow or large.

When a file that is not a spec is read as one, a problem does not quote
a text value from it; like a problem with any spec, it may still name a
key or a short word it did not expect.

A source's words are the spec author's own, and `archgram check` says
whether a file under the project holds them. The names above keep the
usual secret files out, but no list holds every name: in CI, run it
before a step writes a secret into the project's folder
(`actions/checkout` with `persist-credentials: false` keeps the token
out of `.git/config`).

archgram looks a path up, then opens it, as two steps. Another process
changing the files between them could slip a link in; CI and a person
checking a spec run nothing that does, so archgram does not guard
against it.
