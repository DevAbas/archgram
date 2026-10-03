# Security policy

## Supported versions

Only the [latest release](https://github.com/byAbas/archgram/releases/latest)
gets security fixes; an earlier release gets none.

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
theme file and the design token files it names) and writes the diagram.
It runs no scripts, makes no network requests and downloads nothing, at
install time or at run time. Its npm packages have no dependencies and no
install scripts, and are published from GitHub Actions with provenance.
