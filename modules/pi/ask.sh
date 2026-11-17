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
# Started with --no-launcher, as Explain It starts it to say what a session
# works on from that session's own words, it has no tools at all, even
# where there's a launcher: words an agent wrote must never reach a pi that
# can run what the launcher offers. Its prompt then says nothing of tools.
#
# Its own help, in the shape every command of ours takes (docs/commands.md),
# says what it adds to pi and leaves the rest to pi's. The launcher never
# asks it for help, so this answers only a person or an agent.
set -euo pipefail

case "${1:-}" in
-h | --help)
  cat << 'EOF'
pi-ask  pi for the launcher's Ask row

  The launcher speaks pi's RPC protocol over its stdin and
  stdout: a quick question and a short answer each. It
  loads only the extensions the machine names, the digest
  of what's true on it and, where there's a launcher, the
  launcher's tool. Every other word is pi's, as pi --help
  says.

  --no-launcher  no tools at all, the launcher's either

EXAMPLES
  pi-ask  waits for the launcher's questions on stdin

EXIT
  pi's own
EOF
  exit 0
  ;;
*) ;;
esac

launcher=$LAUNCHER_TOOL
if [ "${1:-}" = --no-launcher ]; then
  launcher=
  shift
fi

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
# said here it didn't. Between a list and a run it still said "Let me set
# it to 40%", so it waits for the last call, not the first.
prompt="You answer quick questions in a desktop launcher. One or two plain sentences, no markdown."
if [ -n "$launcher" ]; then
  prompt="$prompt When you have a tool for what's asked, use it first and write nothing until your last call has answered, then say only what was done."
fi
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
if [ -n "$launcher" ]; then
  arguments+=(-e "$launcher" --tools launcher --exclude-tools 'mcp__*')
else
  arguments+=(--no-tools)
fi

# Told to stop, pi winds its extensions down, which took 0.7 to 2.5 s on a
# work machine (2026-11-13), and systemd starts the shell again only once
# every process of its service has gone, so each restart of the shell waited
# on it. With no session there's nothing of pi's to keep, so a stop ends it
# at once. Its stdin is the launcher's, given on, since a command run in the
# background reads nothing otherwise.
asks() {
  "$@" <&0 &
  asking=$!
  trap 'kill -KILL "$asking" 2> /dev/null || true' TERM
  wait "$asking"
}

case $PLAN in
  claude) asks "$PI_CLAUDE" --no-extensions "${arguments[@]}" "$@" ;;
  *)
    if pi --no-extensions --version > /dev/null 2>&1; then
      arguments=(--no-extensions "${arguments[@]}")
    fi
    asks pi "${arguments[@]}" "$@"
    ;;
esac
