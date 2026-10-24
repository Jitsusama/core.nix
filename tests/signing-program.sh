# shellcheck shell=bash
# Runs git's signing program (modules/signing/sign.sh) with a stand-in for
# the hardware's ssh-keygen, so nothing touches a TPM or a Secure Enclave,
# and checks whose key it picks and what it does with what's signed: Joel's
# at a terminal, the agent's over pipes or with no terminal at all, either
# when SIGN_AS says; a signature that verifies is kept, and the empty one a
# Mac's Touch ID key hands back when nobody touches it is taken back, saying
# why; verifying passes straight through; a missing key or an unknown
# SIGN_AS is refused. `script` gives a case a terminal.
# usage: signing-program.sh <sign.sh> <empty-signature.sig>
set -euo pipefail

program=$1
empty=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export WORK=$work
ssh_keygen=$(command -v ssh-keygen)
export SSH_KEYGEN=$ssh_keygen
ssh-keygen -q -t ed25519 -N '' -f "$work/throwaway"
touch "$work/joel-signing" "$work/agent-signing"
cp "$empty" "$work/empty.sig"

# The stand-in keeps the key it was asked to sign with in $WORK/key-used
# and signs with a throwaway key, or, with STAND_IN_SIGNS=empty, leaves the
# empty signature and says it succeeded, as the Touch ID key does.
{
  printf '#!%s\n' "$BASH"
  cat <<'EOF'
if [ "${1:-}" != -Y ] || [ "${2:-}" != sign ]; then
  exec "$SSH_KEYGEN" "$@"
fi
previous=
for argument in "$@"; do
  [ "$previous" = -f ] && printf '%s\n' "$argument" > "$WORK/key-used"
  previous=$argument
done
file=${!#}
if [ "${STAND_IN_SIGNS:-}" = empty ]; then
  cp "$WORK/empty.sig" "$file.sig"
  exit 0
fi
exec "$SSH_KEYGEN" -q -Y sign -n git -f "$WORK/throwaway" "$file"
EOF
} > "$work/stand-in"
chmod +x "$work/stand-in"

# The program as home.nix writes it, with the stand-in as its ssh-keygen.
{
  printf '#!%s\n' "$BASH"
  printf 'set -euo pipefail\n'
  printf 'name=sign real=%q joel_key=%q agent_key=%q makes_keys=se-keys\n' \
    "$work/stand-in" "$work/joel-signing" "$work/agent-signing"
  cat "$program"
} > "$work/sign"
chmod +x "$work/sign"

fails() {
  echo "git's signing program: $1" >&2
  exit 1
}

# Signs $WORK/message as git does, with whatever runs it given first,
# keeping what it said in $WORK/said and its status in $status.
signing="$work/sign -Y sign -n git -f $work/joel-signing $work/message"
sign() {
  rm -f "$work/message.sig" "$work/key-used"
  echo message > "$work/message"
  status=0
  "$@" 2> "$work/said" || status=$?
}
used() { cat "$work/key-used" 2> /dev/null || echo nothing; }

sign script -qec "$signing" /dev/null > /dev/null
[[ $(used) == "$work/joel-signing" ]] || fails "signed at a terminal with $(used), not Joel's key"

sign script -qec "$signing < /dev/null" /dev/null > /dev/null
[[ $(used) == "$work/agent-signing" ]] \
  || fails "a terminal it could open, with none on its input, signed with $(used)"

sign script -qec "echo | $signing" /dev/null > /dev/null
[[ $(used) == "$work/agent-signing" ]] || fails "over a pipe it signed with $(used)"

sign setsid -w "$BASH" -c "$signing < /dev/null"
[[ $(used) == "$work/agent-signing" ]] || fails "with no terminal at all it signed with $(used)"

sign env SIGN_AS=joel setsid -w "$BASH" -c "$signing < /dev/null"
[[ $(used) == "$work/joel-signing" ]] || fails "SIGN_AS=joel signed with $(used)"
((status == 0)) || fails "refused a signature that verifies: $(cat "$work/said")"
[[ -s $work/message.sig ]] || fails "didn't keep a signature that verifies"
"$work/sign" -Y check-novalidate -n git -s "$work/message.sig" < "$work/message" 2>&1 \
  | grep -q '^Good "git" signature' || fails "didn't pass verifying straight to ssh-keygen"

sign env SIGN_AS=joel STAND_IN_SIGNS=empty setsid -w "$BASH" -c "$signing < /dev/null"
((status == 1)) || fails "took the empty signature as signed"
[[ ! -e $work/message.sig ]] || fails "left the empty signature for git"
grep -qF "joel's signature doesn't verify, so nothing was signed." "$work/said" \
  || fails "refused the empty signature without saying why: $(cat "$work/said")"
grep -qF "SIGN_AS=agent" "$work/said" || fails "didn't say the agent's key signs without asking"

sign env SIGN_AS=nobody setsid -w "$BASH" -c "$signing < /dev/null"
((status == 2)) || fails "took SIGN_AS=nobody"
grep -qF "SIGN_AS must be joel or agent, not 'nobody'." "$work/said" \
  || fails "refused SIGN_AS=nobody without saying why: $(cat "$work/said")"

rm "$work/agent-signing"
sign setsid -w "$BASH" -c "$signing < /dev/null"
((status == 1)) || fails "signed with a key that's missing"
grep -qF "$work/agent-signing is missing; se-keys makes it." "$work/said" \
  || fails "refused a missing key without saying what makes it: $(cat "$work/said")"
[[ $(used) == nothing ]] || fails "asked ssh-keygen to sign with a key that's missing"
