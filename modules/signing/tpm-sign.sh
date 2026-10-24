# shellcheck shell=bash disable=SC2154 # The keys' paths come from home.nix's header.
# git's signing program: signs with Joel's key when a person is at the
# keyboard and the agent's when a harness drives git, as home.nix says.
# The header written above this (./home.nix) gives joel_key and agent_key,
# each key's path without its .pub.
#
# git runs it for verifying too, so anything but a signature goes straight
# to ssh-keygen. A signature is made with the chosen key in place of the one
# git names after -f, then checked against the file it signs, and taken back
# when it doesn't verify, as a refused confirmation leaves.

if [ "${1:-}" != "-Y" ] || [ "${2:-}" != "sign" ]; then
  exec ssh-keygen "$@"
fi

signer="${TPM_SIGNER:-}"
if [ -z "$signer" ]; then
  if [ -t 0 ] && { exec 3<>/dev/tty; } 2>/dev/null; then
    exec 3>&-
    signer=joel
  else
    signer=agent
  fi
fi
case "$signer" in
  joel) key="$joel_key.pub" ;;
  agent) key="$agent_key.pub" ;;
  *)
    echo "tpm-sign: TPM_SIGNER must be joel or agent, not '$signer'." >&2
    exit 2
    ;;
esac
if [ ! -f "$key" ]; then
  echo "tpm-sign: $key is missing; docs/installing.md's step 9 makes it." >&2
  exit 1
fi

# The keys are in the TPM's agent, whatever agent SSH itself is using.
export SSH_AUTH_SOCK="${SSH_TPM_AUTH_SOCK:-$XDG_RUNTIME_DIR/ssh-tpm-agent.sock}"

args=()
namespace=
previous=
for argument in "$@"; do
  if [ "$previous" = -f ]; then
    args+=("$key")
  else
    args+=("$argument")
  fi
  [ "$previous" = -n ] && namespace=$argument
  previous=$argument
done

# git signs one file, its last argument; a signature of standard input
# can't be read back to check.
file="${!#}"
if [ ! -f "$file" ] || [ -z "$namespace" ]; then
  exec ssh-keygen "${args[@]}"
fi

ssh-keygen "${args[@]}"
if ! ssh-keygen -Y check-novalidate -n "$namespace" -s "$file.sig" \
  < "$file" > /dev/null 2>&1; then
  rm -f "$file.sig"
  echo "tpm-sign: $signer's signature doesn't verify, so nothing was signed." >&2
  exit 1
fi
