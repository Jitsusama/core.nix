# shellcheck shell=bash
# pi-claude: pi on Joel's own Claude plan, through Claude Code signed in to
# that plan (the claude-code provider, $PROVIDER). A plan's limits are small
# for agent work, so it starts on Sonnet unless given a --model, and stays off
# the mesh (PI_MESH=off), where the machine's holds and questions would start
# turns on that allowance. The login lives in a config folder of its own,
# ~/.claude-personal, apart from any other Claude Code login in ~/.claude,
# and `pi-claude login` signs it in.
#
# Anything pointing Claude Code at an API key, a proxy or a cloud bills that
# rather than the plan, and the provider refuses to start beside it, so none
# of it reaches Claude Code from here.
#
# usage: pi-claude [pi's own arguments]
#        pi-claude login
set -euo pipefail

config=${PI_CLAUDE_CONFIG_DIR:-$HOME/.claude-personal}
away=(
  -u ANTHROPIC_API_KEY
  -u ANTHROPIC_AUTH_TOKEN
  -u ANTHROPIC_BASE_URL
  -u ANTHROPIC_FOUNDRY_API_KEY
  -u CLAUDE_CODE_USE_BEDROCK
  -u CLAUDE_CODE_USE_VERTEX
  -u CLAUDE_CODE_USE_FOUNDRY
)

if [ "${1:-}" = login ]; then
  mkdir -p "$config"
  env "${away[@]}" CLAUDE_CONFIG_DIR="$config" "$CLAUDE" auth login
  exec env "${away[@]}" CLAUDE_CONFIG_DIR="$config" "$CLAUDE" auth status
fi

model=(--model claude-code/claude-sonnet-5)
for argument in "$@"; do
  case $argument in
    --model | --model=*) model=() ;;
  esac
done

exec env "${away[@]}" \
  PI_CLAUDE_CODE_COMMAND="$CLAUDE" \
  PI_CLAUDE_CODE_CONFIG_DIR="$config" \
  PI_MESH=off \
  pi -e "$PROVIDER/src/index.ts" "${model[@]}" "$@"
