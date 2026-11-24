# shellcheck shell=bash
# Puts the key pi-agent-proxy made for OS Login hosts into the person's
# login profile for twelve hours, or takes it out, with the person's own
# gcloud login (src/os_login.rs says why the key exists). The account's
# user units run it: when the proxy writes a new key, every quarter hour
# by the wall clock while it runs, so the key never lapses for long, even
# past a night asleep, and as the proxy stops, since its private half
# goes with it.
#
#   pi-agent-proxy-os-login register|remove PUBLIC-KEY-FILE
#
# It runs the gcloud on the user manager's PATH, the one the person's
# login was made with. A machine with no gcloud registers nothing, and
# says so in the journal.

mode=${1:?register or remove}
key=${2:?the public file of the key}

# Each call is bounded, so a hung network can't hold the next renewal
# up: the timer starts the unit again only once this one ends.
cloud() {
  timeout 60 gcloud compute os-login ssh-keys "$@" --quiet --verbosity=error
}

if [[ $mode != register && $mode != remove ]]; then
  echo "pi-agent-proxy-os-login: $mode is neither register nor remove" >&2
  exit 64
fi
if ! command -v gcloud >/dev/null; then
  echo "pi-agent-proxy-os-login: no gcloud on PATH, so it does nothing"
  exit 0
fi
if [[ ! -s $key ]]; then
  echo "pi-agent-proxy-os-login: no key at $key, so it does nothing"
  exit 0
fi
fingerprint=$(ssh-keygen -lf "$key" | cut -d' ' -f2)

if [[ $mode == register ]]; then
  # update moves the end of a key already there; add puts in one that
  # isn't, as each new proxy's is, or one that lapsed.
  if cloud update --key-file="$key" --ttl=12h >/dev/null 2>&1; then
    echo "pi-agent-proxy-os-login: $fingerprint renewed for 12 hours"
  elif cloud add --key-file="$key" --ttl=12h >/dev/null; then
    echo "pi-agent-proxy-os-login: $fingerprint registered for 12 hours"
  else
    echo "pi-agent-proxy-os-login: $fingerprint couldn't be registered; trying again soon" >&2
    exit 1
  fi
else
  # The file stays, so no event of its going reaches the path unit as
  # the next proxy writes its own; that next key replaces it.
  if cloud remove --key-file="$key" >/dev/null; then
    echo "pi-agent-proxy-os-login: $fingerprint removed"
  else
    echo "pi-agent-proxy-os-login: $fingerprint couldn't be removed; it lapses within 12 hours"
  fi
fi
