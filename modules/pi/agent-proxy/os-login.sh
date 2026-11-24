# shellcheck shell=bash
# Puts the key pi-agent-proxy made for OS Login hosts into the person's
# login profile for twelve hours, or takes it out, with the person's own
# gcloud login (src/os_login.rs says why the key exists). The account's
# user units run it: when the proxy writes a new key, every four hours
# while it runs so the key never lapses, and as the proxy stops, since
# its private half goes with it.
#
#   pi-agent-proxy-os-login register|remove PUBLIC-KEY-FILE
#
# It runs the gcloud on the account's PATH, the one the person's login
# was made with. A machine with no gcloud registers nothing, and says so.

mode=${1:?register or remove}
key=${2:?the public file of the key}

if ! command -v gcloud >/dev/null; then
  echo "pi-agent-proxy-os-login: no gcloud on PATH, so no OS Login key is ${mode}ed"
  exit 0
fi
if [[ ! -s $key ]]; then
  echo "pi-agent-proxy-os-login: no key at $key, so none is ${mode}ed"
  exit 0
fi
fingerprint=$(ssh-keygen -lf "$key" | cut -d' ' -f2)

case $mode in
  register)
    # update moves the end of a key already there; add puts in one that
    # isn't, as each new proxy's is.
    if gcloud compute os-login ssh-keys update --key-file="$key" --ttl=12h \
      --quiet --verbosity=error >/dev/null 2>&1; then
      echo "pi-agent-proxy-os-login: $fingerprint renewed for 12 hours"
    else
      gcloud compute os-login ssh-keys add --key-file="$key" --ttl=12h \
        --quiet --verbosity=error >/dev/null
      echo "pi-agent-proxy-os-login: $fingerprint registered for 12 hours"
    fi
    ;;
  remove)
    # Bounded, since the proxy stops as the machine does, when the
    # network may already be gone; a key left lapses within 12 hours.
    if timeout 20 gcloud compute os-login ssh-keys remove --key-file="$key" \
      --quiet --verbosity=error >/dev/null; then
      echo "pi-agent-proxy-os-login: $fingerprint removed"
    else
      echo "pi-agent-proxy-os-login: $fingerprint couldn't be removed; it lapses within 12 hours"
    fi
    rm -f "$key"
    ;;
  *)
    echo "pi-agent-proxy-os-login: $mode is neither register nor remove" >&2
    exit 64
    ;;
esac
