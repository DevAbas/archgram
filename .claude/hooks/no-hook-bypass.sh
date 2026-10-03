#!/bin/sh
# A Claude Code PreToolUse hook (.claude/settings.json): refuses a shell
# command that would skip the git hooks, which check a branch's name before
# each commit and push (CONTRIBUTING.md, Setting up). git lets anyone skip
# them, with --no-verify, `git commit -n`, or another core.hooksPath; an
# agent is asked to fix what they report instead. CI checks the same rules
# after, whatever runs here.
#
# Claude Code gives the tool call as JSON on stdin; exit 2 refuses it and
# shows stderr to Claude (https://code.claude.com/docs/en/hooks.md). The
# command is read with sed, so the hook needs nothing jq would.
command=$(tr -d '\n' | sed -nE 's/.*"command"[[:space:]]*:[[:space:]]*"(([^"\\]|\\.)*)".*/\1/p')

case $command in
  *git*) ;;
  *) exit 0 ;;
esac

refuse() {
  printf '%s\n' "This command would skip the git hooks ($1). They check the branch's name before a commit and a push; fix what they report instead (sh scripts/check-change branch)." >&2
  exit 2
}

case $command in
  *--no-verify*) refuse '--no-verify' ;;
esac
printf '%s\n' "$command" |
  grep -Eq 'git[[:space:]]+commit([[:space:]]+-[^[:space:]"\\]+)*[[:space:]]+-[a-zA-Z]*n[a-zA-Z]*([[:space:]]|$)' &&
  refuse 'git commit -n'
case $command in
  *core.hooksPath*)
    printf '%s\n' "$command" | grep -Eq 'config[[:space:]]+core\.hooksPath[[:space:]]+\.githooks([[:space:]]|$)' ||
      refuse 'core.hooksPath'
    ;;
esac
exit 0
