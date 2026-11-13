# shellcheck shell=bash disable=SC2154 # joel_key and agent_key come from home.nix.
# Makes the signing keys home.nix names, once, and is safe to run again:
# the agents' key as a plain file, and Joel's on the YubiKey that's plugged
# in. A YubiKey that already holds Joel's key, from an earlier install, has
# its handle downloaded rather than a key made again, since a new one would
# leave every register the old one is in holding a key nothing signs with.
# It ends by printing the public keys, which belong in
# jitsusama.signing.allowedSigners.
#
# It takes no words, and refuses any before it does anything, so --help or
# a typo never makes a key or asks for the PIN and a touch.
if [ "$#" -gt 0 ]; then
  case $1 in
    -h | --help)
      cat << 'EOF'
yubikey-keys  makes the signing keys, once, safe to run again

EXAMPLES
  yubikey-keys   make what's missing, then print the public keys

EXIT
  0   every key is there
  1   a key couldn't be made or downloaded
  64  asked wrongly
EOF
      exit 0
      ;;
    *)
      printf '%s\n' "yubikey-keys takes no words" "  try  yubikey-keys --help" >&2
      exit 64
      ;;
  esac
fi

# The YubiKey's own name for Joel's key, apart from the login key's spare,
# which is plain ssh:.
readonly application=ssh:joel-signing
comment="$USER@$(hostname)"
readonly comment

make_agent_key() {
  if [ -f "$agent_key" ]; then
    echo "agents' key: present"
    return
  fi
  mkdir -p "$(dirname "$agent_key")"
  ssh-keygen -q -t ed25519 -N '' -C "$comment agents" -f "$agent_key"
  echo "agents' key: made"
}

# Downloads Joel's key's handle from the YubiKey, if it holds one, into
# place, and fails when it doesn't.
download_joel_key() {
  local made found
  made=$(mktemp -d)
  # shellcheck disable=SC2064 # The folder is known now and should go then.
  trap "rm -rf '$made'" RETURN
  (cd "$made" && ssh-keygen -q -K -N '' > /dev/null 2>&1) || return 1
  found=$(find "$made" -name "id_*_rk_${application#ssh:}" -print -quit)
  [ -n "$found" ] || return 1
  install -m 600 "$found" "$joel_key"
  install -m 644 "$found.pub" "$joel_key.pub"
}

make_joel_key() {
  if [ -f "$joel_key" ]; then
    echo "Joel's key: present"
    return
  fi
  mkdir -p "$(dirname "$joel_key")"
  echo "Joel's key: looking on the YubiKey; type its PIN and touch it when it blinks."
  if download_joel_key; then
    echo "Joel's key: downloaded from the YubiKey"
    return
  fi
  echo "Joel's key: making it on the YubiKey; type its PIN and touch it again."
  ssh-keygen -q -t ed25519-sk -O resident -O verify-required \
    -O "application=$application" -N '' -C "$comment signing" -f "$joel_key"
  echo "Joel's key: made"
}

main() {
  make_agent_key
  make_joel_key
  echo
  echo "public keys, which belong in jitsusama.signing.allowedSigners:"
  local key public
  for key in "$joel_key" "$agent_key"; do
    public=$(cut -d' ' -f1-2 "$key.pub")
    echo "  $public"
  done
}

main
