# shellcheck shell=bash
# Runs pi-ask (modules/pi/ask.nix) as two machines set it up, over stand-ins
# for pi and pi-claude that write down how they were started. On pi's own
# plan, pi is started speaking RPC with nothing loaded but the extensions the
# machine names, on the model it names; on the Claude plan, pi-claude is
# started the same way instead, and pi is not. On a device whose extensions
# are managed centrally, where pi refuses --no-extensions, pi-ask leaves the
# flag out and loads the machine's extensions all the same.
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

# A stand-in pi that writes down how it was started, unless it was only
# asked its version; a managed one refuses --no-extensions, as a device's
# centrally managed pi does.
printf '#!%s\n' "$BASH" > "$work/bin/pi"
cat >> "$work/bin/pi" << 'EOF'
for argument in "$@"; do
  if [ -n "${MANAGED:-}" ] && [ "$argument" = --no-extensions ]; then
    echo "pi: this device has managed Pi extensions." >&2
    exit 1
  fi
done
for argument in "$@"; do
  if [ "$argument" = --version ]; then
    echo 1.4.0
    exit 0
  fi
done
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
MANAGED=1 "$on_pi"
has "started pi"
lacks "arg --no-extensions"
follows --mode rpc
follows --model anthropic/claude-haiku-4-5
[ "$(grep -cxF -- "arg -e" "$said")" = 2 ] ||
  fail "on a managed device, the machine's extensions weren't loaded"

: > "$said"
"$on_claude"
has "started pi-claude"
lacks "started pi"
follows --mode rpc
follows --model claude-code/claude-haiku-4-5-20251001
has "arg --no-extensions"
lacks "arg -e"
