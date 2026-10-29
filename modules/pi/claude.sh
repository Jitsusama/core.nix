# shellcheck shell=bash
# pi-claude: pi on Joel's own Claude plan, through Claude Code signed in to
# that plan (the claude-code provider, $PROVIDER). A plan's limits are small
# for agent work, so it starts on $PLAN_MODEL unless given a --model. It is
# never on the work mesh: on a machine that joins a realm for the plan
# ($PLAN_REALM), it is that realm's session, keeping that realm's folders
# apart from those of the machine's home realm ($MACHINE_REALM), and
# elsewhere it is on no mesh (PI_MESH=off). The login lives in a config
# folder of its own, ~/.claude-personal, apart from any other Claude Code
# login in ~/.claude, and `pi-claude login` signs it in. On a machine whose only login is the
# plan's ($OWN_LOGIN, or ~/.claude named in PI_CLAUDE_CONFIG_DIR), no folder
# is named to Claude Code at all: on macOS a named folder is a keychain entry
# of its own, so naming even the default one leaves its login unseen.
#
# Anything pointing Claude Code at an API key, a proxy or a cloud bills that
# rather than the plan, and the provider refuses to start beside it, so none
# of it reaches Claude Code from here.
#
# usage: pi-claude [pi's own arguments]
#        pi-claude login
set -euo pipefail

if [ -n "$OWN_LOGIN" ]; then
  config=${PI_CLAUDE_CONFIG_DIR:-$HOME/.claude}
else
  config=${PI_CLAUDE_CONFIG_DIR:-$HOME/.claude-personal}
fi
away=(
  -u ANTHROPIC_API_KEY
  -u ANTHROPIC_AUTH_TOKEN
  -u ANTHROPIC_BASE_URL
  -u ANTHROPIC_FOUNDRY_API_KEY
  -u CLAUDE_CODE_USE_BEDROCK
  -u CLAUDE_CODE_USE_VERTEX
  -u CLAUDE_CODE_USE_FOUNDRY
)
# Claude Code's own folder is left unnamed, so its own login is the one read.
if [ "${config%/}" = "$HOME/.claude" ]; then
  named=()
  provided=()
else
  named=(CLAUDE_CONFIG_DIR="$config")
  provided=(PI_CLAUDE_CODE_CONFIG_DIR="$config")
fi

if [ "${1:-}" = login ]; then
  mkdir -p "$config"
  env "${away[@]}" "${named[@]}" "$CLAUDE" auth login
  exec env "${away[@]}" "${named[@]}" "$CLAUDE" auth status
fi

model=(--model "claude-code/$PLAN_MODEL")
if [ -n "$PLAN_REALM" ]; then
  mesh=(MYCELIUM_REALM="$PLAN_REALM" PI_HOME_REALM="$MACHINE_REALM")
else
  mesh=(PI_MESH=off)
fi
# Said to pi only as what they mean.
unset PLAN_MODEL PLAN_REALM MACHINE_REALM
for argument in "$@"; do
  case $argument in
    --model | --model=*) model=() ;;
  esac
done

exec env "${away[@]}" \
  PI_CLAUDE_CODE_COMMAND="$CLAUDE" \
  "${provided[@]}" \
  "${mesh[@]}" \
  pi -e "$PROVIDER/src/index.ts" "${model[@]}" "$@"
