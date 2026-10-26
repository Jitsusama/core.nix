# shellcheck shell=bash
# Runs pi-ask (modules/pi/ask.nix) as two machines set it up, over stand-ins
# for pi and pi-claude that write down how they were started. On pi's own
# plan, pi is started speaking RPC with nothing loaded but the extensions the
# machine names, on the model it names; on the Claude plan, pi-claude is
# started the same way instead, and pi is not.
# usage: pi-ask-answers-on-the-machines-plan.sh <pi-ask on pi's plan>
#          <pi-ask on the Claude plan> <the stand-in pi-claude>
set -euo pipefail

on_pi=$1
on_claude=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
said=$work/said

fail() {
  echo "$1"
  cat "$said" 2> /dev/null || true
  exit 1
}

printf '#!%s\n' "$BASH" > "$work/bin/pi"
cat >> "$work/bin/pi" << 'EOF'
{
  echo "started pi"
  printf 'arg %s\n' "$@"
} > "$SAID"
EOF
chmod +x "$work/bin/pi"
export PATH=$work/bin:$PATH SAID=$said

has() { grep -qxF -- "$1" "$said" || fail "it wasn't given $1"; }
lacks() { ! grep -qxF -- "$1" "$said" || fail "it was given $1"; }
follows() {
  grep -A1 -xF -- "arg $1" "$said" | grep -qxF -- "arg $2" ||
    fail "$1 wasn't followed by $2"
}

"$on_pi" --verbose
has "started pi"
for argument in --mode --no-session --no-tools --no-extensions --no-skills \
  --no-context-files --no-prompt-templates --verbose; do
  has "arg $argument"
done
follows --mode rpc
follows --thinking off
follows --model anthropic/claude-haiku-4-5
grep -A1 -xF -- "arg -e" "$said" | grep -qxF "arg /extensions/a-proxy" ||
  fail "the machine's first extension wasn't loaded"
grep -A1 -xF -- "arg -e" "$said" | grep -qxF "arg /extensions/another" ||
  fail "the machine's second extension wasn't loaded"
[ "$(grep -cxF -- "arg -e" "$said")" = 2 ] || fail "it loaded more than the machine named"

: > "$said"
"$on_claude"
has "started pi-claude"
lacks "started pi"
follows --mode rpc
follows --model claude-code/claude-haiku-4-5-20251001
lacks "arg -e"
