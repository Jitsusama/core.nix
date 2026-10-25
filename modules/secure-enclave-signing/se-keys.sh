# shellcheck shell=bash disable=SC2154 # provider and identities come from home.nix.
# Brings this Mac's Secure Enclave to the signing identities home.nix names,
# and is safe to run again. Making a key is a deliberate act, not a side
# effect of a switch: an enclave key can't be made again from anything, so a
# switch only says when one is missing.
#
# home.nix gives it, through the environment, the provider OpenSSH reaches
# the enclave through, as $provider, and the file of identities, as
# $identities: one a line, each its label, protection, name and email, separated by tabs, the
# email empty when there's none. SE_KEYS_CMD chooses what it does: ensure
# (the default) or status.

readonly sc=/usr/sbin/sc_auth
readonly kg=/usr/bin/ssh-keygen
readonly dest="$HOME/.ssh"

list_ssh() { "$sc" list-ctk-identities -t ssh | tail -n +2; }
fp_of() { list_ssh | awk -v l="$1" '$4 == l { print $2 }'; }
count() { list_ssh | grep -c . || true; }

ensure_identity() {
  local label="$1" protection="$2" name="$3" email="$4"
  if [ -n "$(fp_of "$label")" ]; then
    echo "  identity $label: present ($protection)"
    return 0
  fi
  echo "  identity $label: creating ($protection)"
  local made=(-l "$label" -k p-256-ne -t "$protection" -N "$name")
  [ -n "$email" ] && made+=(-E "$email")
  "$sc" create-ctk-identity "${made[@]}"
}

# ssh-keygen -K writes every resident credential to the same filename and
# asks before overwriting each (Apple FB21992630). Answering y k-1 times and
# then n leaves credential number k, so this walks k until the fingerprint
# matches the identity wanted.
export_handle() {
  local label="$1" want n k tmp fp
  want="$(fp_of "$label")"
  if [ -z "$want" ]; then
    echo "  no identity $label" >&2
    return 1
  fi

  if [ -f "$dest/$label.pub" ] &&
    [ "$("$kg" -lf "$dest/$label.pub" | awk '{print $2}')" = "$want" ]; then
    echo "  handle   $label: present"
    return 0
  fi

  n="$(count)"
  for k in $(seq 1 "$n"); do
    tmp="$(mktemp -d)"
    {
      if [ "$k" -gt 1 ]; then yes y | head -n "$((k - 1))"; fi
      yes n | head -n "$((n - k + 1))"
    } | (
      cd "$tmp" &&
        SSH_ASKPASS_REQUIRE=force SSH_ASKPASS=/usr/bin/true \
          "$kg" -K -w "$provider" -N "" > /dev/null 2>&1
    ) || true

    if [ -f "$tmp/id_ecdsa_sk_rk.pub" ]; then
      fp="$("$kg" -lf "$tmp/id_ecdsa_sk_rk.pub" | awk '{print $2}')"
      if [ "$fp" = "$want" ]; then
        install -m 600 "$tmp/id_ecdsa_sk_rk" "$dest/$label"
        install -m 644 "$tmp/id_ecdsa_sk_rk.pub" "$dest/$label.pub"
        rm -rf "$tmp"
        echo "  handle   $label: installed"
        return 0
      fi
    fi
    rm -rf "$tmp"
  done
  echo "  could not export a handle for $label" >&2
  return 1
}

ensure() {
  local label protection name email labels=()
  # Never let these sign in: an identity that may is reported to turn the
  # login window into a prompt for a PIN that doesn't exist.
  "$sc" pairing_ui -s disable > /dev/null 2>&1 || true
  # The identities come in on their own descriptor, so nothing the loop runs
  # reads them from its input.
  while IFS=$'\t' read -r -u 3 label protection name email; do
    ensure_identity "$label" "$protection" "$name" "$email"
    export_handle "$label"
    labels+=("$label")
  done 3< "$identities"
  echo
  echo "public keys, which belong in jitsusama.signing.allowedSigners:"
  for label in "${labels[@]}"; do
    [ -f "$dest/$label.pub" ] && echo "  $(cut -d' ' -f1-2 "$dest/$label.pub")"
  done
}

main() {
  case "${SE_KEYS_CMD:-ensure}" in
    ensure) ensure ;;
    status) list_ssh ;;
    *)
      echo "usage: SE_KEYS_CMD=ensure|status se-keys" >&2
      exit 64
      ;;
  esac
}

main
