# shellcheck shell=bash
# Runs pi-claude (modules/pi/claude.nix) over stand-ins for pi and Claude
# Code that write down how they were started. pi gets the provider, Sonnet
# unless a model is asked for, the mesh off, and Claude Code and the plan's
# own login folder, with nothing in its environment that would bill
# something other than the plan; `pi-claude login` signs Claude Code in to
# that folder and nowhere else. Claude Code's own folder is never named.
# On a machine whose own login is the plan's, that login is the one read.
# usage: pi-claude-keeps-to-the-plan.sh <pi-claude> <the stand-in Claude Code>
#          <pi-claude on the machine's own login>
set -euo pipefail

program=$1
claude=$2
own=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin" "$work/home"
export HOME=$work/home
said=$work/said

fail() {
  echo "$1"
  cat "$said" 2> /dev/null || true
  exit 1
}

printf '#!%s\n' "$BASH" > "$work/bin/pi"
cat >> "$work/bin/pi" << 'EOF'
{
  printf 'arg %s\n' "$@"
  env | sed 's/^/env /'
} > "$SAID"
EOF
chmod +x "$work/bin/pi"
export PATH=$work/bin:$PATH SAID=$said
export ANTHROPIC_BASE_URL=https://proxy.example ANTHROPIC_API_KEY=key CLAUDE_CODE_USE_BEDROCK=1

has() { grep -qxF -- "$1" "$said" || fail "pi wasn't given $1"; }
lacks() { ! grep -q -- "$1" "$said" || fail "pi was given $1"; }

"$program" --continue
has "env PI_MESH=off"
has "env PI_CLAUDE_CODE_COMMAND=$claude"
has "env PI_CLAUDE_CODE_CONFIG_DIR=$HOME/.claude-personal"
has "arg claude-code/claude-sonnet-5"
has "arg --continue"
lacks "^env ANTHROPIC_"
lacks "^env CLAUDE_CODE_USE_"
provider=$(grep -A1 -xF "arg -e" "$said" | sed -n '2s/^arg //p')
[ -f "$provider" ] || fail "the provider pi was given, $provider, isn't there"

"$program" --model claude-code/claude-haiku-4-5-20251001
lacks "claude-sonnet-5"
has "arg claude-code/claude-haiku-4-5-20251001"

: > "$said"
"$program" login > /dev/null
has "arg login"
has "arg status"
has "env CLAUDE_CONFIG_DIR=$HOME/.claude-personal"
lacks "^env ANTHROPIC_"
[ -d "$HOME/.claude-personal" ] || fail "login didn't make the plan's login folder"

# A machine whose only login is the plan's names Claude Code's own folder,
# which is then named to Claude Code not at all, so its own login is read.
: > "$said"
PI_CLAUDE_CONFIG_DIR=$HOME/.claude "$program"
lacks "^env PI_CLAUDE_CODE_CONFIG_DIR="
lacks "^env CLAUDE_CONFIG_DIR="
: > "$said"
PI_CLAUDE_CONFIG_DIR=$HOME/.claude/ "$program" login > /dev/null
has "arg login"
lacks "^env CLAUDE_CONFIG_DIR="

: > "$said"
"$own"
has "env PI_CLAUDE_CODE_COMMAND=$claude"
lacks "^env PI_CLAUDE_CODE_CONFIG_DIR="
lacks "^env CLAUDE_CONFIG_DIR="
