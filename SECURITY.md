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
  archgram runs in, is refused, and so is a source on the way to which
  there is one;
- a theme's token files only under the theme file's folder, and a
  spec's sources only under the folder archgram runs in, judged by the
  path's text before the disk is looked at, so a spec cannot learn
  whether a file outside the project exists;
- never inside `.git`.

A problem names where it is in the spec, never a text it quotes from a
file read as a spec by mistake.

A source's words are the spec author's own, and `archgram check` says
whether a file under the project holds them. In CI, run it before a step
writes a secret into the project's folder (a `.env` file, a token in
`.git/config`; `actions/checkout` with `persist-credentials: false`
keeps it out).
