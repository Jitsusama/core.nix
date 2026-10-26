# shellcheck shell=bash
# pi-ask: pi as the launcher's Ask row asks it, a quick question and a short
# answer. The launcher keeps one running per login and speaks pi's RPC
# protocol over its stdin and stdout, starting each question with a fresh
# session, so pi's start, about a second and a half, is paid once.
#
# It loads nothing it doesn't need to answer: no tools, skills, context files
# or prompt templates, no session file, no thinking, and no extensions but the
# ones the machine names ($EXTENSIONS, one per line), such as the one a work
# machine reaches its models through. Which plan answers is the machine's one
# setting ($PLAN): "pi" for pi's own provider, or "claude" for Joel's own
# Claude plan through pi-claude ($PI_CLAUDE), on $MODEL either way.
#
# A device whose pi extensions are managed centrally has a pi that refuses
# --no-extensions, since it delivers the managed ones itself. pi is asked once
# whether it takes the flag, which works wherever the managed list is kept,
# and where it doesn't, pi loads what it would; with no session it stays off
# the mesh all the same.
#
# usage: pi-ask [more of pi's arguments]
set -euo pipefail

prompt="You answer quick questions in a desktop launcher. One or two plain sentences, no markdown."
arguments=(
  --mode rpc
  --no-session
  --no-tools
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

case $PLAN in
  claude) exec "$PI_CLAUDE" --no-extensions "${arguments[@]}" "$@" ;;
  *)
    if pi --no-extensions --version > /dev/null 2>&1; then
      arguments=(--no-extensions "${arguments[@]}")
    fi
    exec pi "${arguments[@]}" "$@"
    ;;
esac
