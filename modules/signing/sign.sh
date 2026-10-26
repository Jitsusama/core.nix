# shellcheck shell=bash disable=SC2154 # The signer's settings come from home.nix's header.
# git's signing program: signs with Joel's key when a person is at the
# keyboard and the agent's when a harness drives git, as home.nix says, and
# checks what it signed. The header written above this (./home.nix) gives
# name, the program's own name; real, the ssh-keygen that signs; joel_key and
# agent_key, what ssh-keygen's -f takes for each key; and makes_keys, what
# makes a key that's missing. It may export what that ssh-keygen needs to
# reach the keys, too, and defines agent_environment, which exports what it
# needs to reach the agents' key alone.
#
# git runs it for verifying as well, so anything but a signature goes
# straight to ssh-keygen. A signature is made with the chosen key in place of
# the one git names after -f, then checked against the file it signs, and
# taken back when it doesn't verify: a refused confirmation leaves one, and a
# Mac's Touch ID key hands back a well-formed signature with nothing in it
# when nobody touches the sensor, and says it succeeded.

# Picks who signs, into signer, and their key, into key: SIGN_AS when it's
# set, or else Joel when standard input is a terminal and the controlling
# terminal opens, since a harness started from a terminal keeps that
# terminal while it drives git over pipes.
pick_signer() {
  signer="${SIGN_AS:-}"
  if [ -z "$signer" ]; then
    if [ -t 0 ] && { exec 3<>/dev/tty; } 2>/dev/null; then
      exec 3>&-
      signer=joel
    else
      signer=agent
    fi
  fi

  case "$signer" in
    joel) key=$joel_key ;;
    agent)
      key=$agent_key
      agent_environment
      ;;
    *)
      echo "$name: SIGN_AS must be joel or agent, not '$signer'." >&2
      exit 2
      ;;
  esac
  if [ ! -f "$key" ]; then
    echo "$name: $key is missing; $makes_keys makes it." >&2
    exit 1
  fi
}

# Copies git's arguments into args with the key after -f swapped for the
# signer's, and keeps the namespace after -n, which the check needs.
swap_key() {
  args=()
  namespace=
  local previous='' argument
  for argument in "$@"; do
    if [ "$previous" = -f ]; then
      args+=("$key")
    else
      args+=("$argument")
    fi
    [ "$previous" = -n ] && namespace=$argument
    previous=$argument
  done
}

# Fails the signing, and leaves no signature for git, when the one written
# doesn't verify.
check_signature() {
  if ! "$real" -Y check-novalidate -n "$namespace" -s "$file.sig" < "$file" \
    > /dev/null 2>&1; then
    rm -f "$file.sig"
    echo "$name: $signer's signature doesn't verify, so nothing was signed." >&2
    if [ "$signer" = joel ]; then
      echo "$name: SIGN_AS=agent signs with the agents' key, which asks nothing." >&2
    fi
    exit 1
  fi
}

main() {
  if [ "${1:-}" != -Y ] || [ "${2:-}" != sign ]; then
    exec "$real" "$@"
  fi

  pick_signer
  swap_key "$@"

  # git signs one file, its last argument; a signature of standard input
  # can't be read back to check.
  file="${!#}"
  if [ ! -f "$file" ] || [ -z "$namespace" ]; then
    exec "$real" "${args[@]}"
  fi

  "$real" "${args[@]}"
  check_signature
}

main "$@"
