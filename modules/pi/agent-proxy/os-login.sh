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
# It registers only the key the proxy beside the file holds, asked of
# the proxy's own socket, so neither a previous proxy's key left in the
# file nor one written there by anything else is ever registered.
#
# It runs the gcloud on the user manager's PATH, the one the person's
# login was made with. A machine with no gcloud, or a gcloud with no
# login, registers nothing, and warns so in the journal, naming what
# would; neither fails the unit, as neither passes by trying again.
# A failed call does, and the unit tries again shortly.
#
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
pi-agent-proxy-os-login  keep agents' OS Login key

  register puts the proxy's OS Login key in the person's
  login profile for twelve hours, or renews it there, once
  the proxy says it holds it; remove takes it out, keeping
  the key's file. Each uses
  the person's own gcloud login. No gcloud, or no login,
  is warned of and registers nothing.

EXAMPLES
  pi-agent-proxy-os-login register \
    /run/user/1000/pi-agent/os-login.pub

EXIT
  0   done, or nothing it can do
  1   gcloud couldn't register the key
  64  neither register nor remove
EOF
  exit 0
  ;;
*) ;;
esac

mode=${1:?register or remove}
key=${2:?the public file of the key}

if [[ $mode != register && $mode != remove ]]; then
  echo "pi-agent-proxy-os-login: $mode is neither register nor remove" >&2
  exit 64
fi

# Each call is bounded, so a hung network can't hold the next renewal
# up: the timer starts the unit again only once this one ends. A
# removal is bounded tighter, since it holds the proxy's stop, and so a
# logout; a key it can't take out lapses on its own.
bound=60
[[ $mode == register ]] || bound=15
cloud() {
  timeout "$bound" gcloud compute os-login ssh-keys "$@" --quiet --verbosity=error
}

# Whether the proxy whose socket sits beside the key lists the key in
# the file: the key is the file's second field, and the proxy lists it
# whole, without the time it was made.
agent=${key%/*}/agent.sock
held() {
  local kind blob rest listed
  read -r kind blob rest <"$key" || return 1
  listed=$(SSH_AUTH_SOCK=$agent timeout 10 ssh-add -L 2>/dev/null) || return 1
  [[ -n $blob && $listed == *"$kind $blob "* ]]
}
if ! command -v gcloud >/dev/null; then
  echo "<4>pi-agent-proxy-os-login: no gcloud on the user manager's PATH, so agents' OS Login" \
    "logins fail; set jitsusama.pi.agentProxy.osLoginKnownHosts to null where none is wanted"
  exit 0
fi
# The timer can fire as the proxy starts, a moment before it writes its
# key, while the file still holds the previous proxy's, and the path's
# event for the new key then joins this run rather than starting
# another; so a register waits a few seconds for the proxy to hold the
# key in the file.
if [[ $mode == register ]]; then
  for _ in {1..25}; do
    [[ -s $key ]] && held && break
    sleep 0.2 # The proxy writes its key in this while.
  done
  if ! [[ -s $key ]] || ! held; then
    echo "pi-agent-proxy-os-login: the proxy holds no key at $key, so it does nothing"
    exit 0
  fi
elif [[ ! -s $key ]]; then
  echo "pi-agent-proxy-os-login: no key at $key, so it does nothing"
  exit 0
fi
# A gcloud that can't say fails as one with no login would: either way
# there's nothing to register with.
login=$(timeout 60 gcloud config get-value account --quiet 2>/dev/null) || login=
if [[ -z $login ]]; then
  echo "<4>pi-agent-proxy-os-login: gcloud has no login, so agents' OS Login logins fail;" \
    "gcloud auth login gives it one"
  exit 0
fi
# ssh-keygen says the bits, the fingerprint, then the comment.
fingerprint=$(ssh-keygen -lf "$key")
fingerprint=${fingerprint#* }
fingerprint=${fingerprint%% *}

if [[ $mode == register ]]; then
  # update moves the end of a key already there; add puts in one that
  # isn't, as each new proxy's is, or one that lapsed.
  if cloud update --key-file="$key" --ttl=12h >/dev/null 2>&1; then
    echo "pi-agent-proxy-os-login: $fingerprint renewed for 12 hours"
  elif cloud add --key-file="$key" --ttl=12h >/dev/null; then
    echo "pi-agent-proxy-os-login: $fingerprint registered for 12 hours"
  else
    echo "<3>pi-agent-proxy-os-login: $fingerprint couldn't be registered; trying again soon," \
      "and gcloud auth login renews a lapsed login" >&2
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
