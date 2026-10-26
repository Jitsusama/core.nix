# shellcheck shell=bash
# pi-ask: pi as the launcher's Ask row asks it, a quick question and a short
# answer. The launcher speaks pi's RPC protocol over its stdin and stdout,
# one question to each, and keeps the next started before it's asked, so
# pi's start, about a second and a half, is never waited on.
#
# It loads nothing it doesn't need to answer: no skills, context files or
# prompt templates, no session file, no thinking, and no extensions but the
# ones the machine names ($EXTENSIONS, one per line), such as the one a work
# machine reaches its models through, and the digest that tells it what's
# true on the machine as each question starts ($DIGEST). Its one tool,
# where there's a launcher to reach ($LAUNCHER_TOOL, through $ASK_LAUNCHER),
# opens and does what the launcher offers; anywhere else it has none, and no
# MCP server's either. Which plan answers is the machine's one setting
# ($PLAN): "pi" for pi's own provider, or "claude" for Joel's own Claude plan
# through pi-claude ($PI_CLAUDE), on $MODEL either way.
#
# The launcher starts it from a user service, whose PATH isn't a login
# shell's, so what an extension runs, such as the program a work machine's
# proxy gets its token from, can be missing from it. The folders the
# machine names ($ASK_PATH, one per line)
# go ahead of the PATH it was started with, in the order named.
#
# A device whose pi extensions are managed centrally has a pi that refuses
# --no-extensions, since it delivers the managed ones itself. pi is asked once
# whether it takes the flag, which works wherever the managed list is kept,
# and where it doesn't, pi loads what it would; with no session it stays off
# the mesh all the same.
#
# usage: pi-ask [more of pi's arguments]
set -euo pipefail

ahead=
while IFS= read -r folder; do
  if [ -n "$folder" ]; then
    ahead=$ahead$folder:
  fi
done <<< "$ASK_PATH"
PATH=$ahead$PATH

# With the launcher's tool, the model wrote "I'll reboot the laptop for you"
# before the tool offered a reboot as a row: words ahead of a tool's answer
# can promise what isn't done. Said in the tool's guidance it still did;
# said here it didn't.
prompt="You answer quick questions in a desktop launcher. One or two plain sentences, no markdown. When you have a tool for what's asked, call it first and write nothing before its answer, then say only what it did."
arguments=(
  --mode rpc
  --no-session
  --no-skills
  --no-context-files
  --no-prompt-templates
  --thinking off
  --model "$MODEL"
  --system-prompt "$prompt"
)
while IFS= read -r extension; do
  if [ -n "$extension" ]; then
    arguments+=(-e "$extension")
  fi
done <<< "$EXTENSIONS"
arguments+=(-e "$DIGEST")
if [ -n "$LAUNCHER_TOOL" ]; then
  arguments+=(-e "$LAUNCHER_TOOL" --tools launcher --exclude-tools 'mcp__*')
else
  arguments+=(--no-tools)
fi

case $PLAN in
  claude) exec "$PI_CLAUDE" --no-extensions "${arguments[@]}" "$@" ;;
  *)
    if pi --no-extensions --version > /dev/null 2>&1; then
      arguments=(--no-extensions "${arguments[@]}")
    fi
    exec pi "${arguments[@]}" "$@"
    ;;
esac
