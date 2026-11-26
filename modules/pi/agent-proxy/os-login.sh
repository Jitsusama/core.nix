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
# It registers only the key the proxy beside the file lists under its
# own OS Login comment, asked of the proxy's own socket, and only while
# the file names that key; what it hands gcloud is the proxy's line,
# copied to a file of its own beside the key, so neither a previous proxy's key left in
# the file nor another key the proxy holds, such as the builders', is ever
# registered, and the proxy writing its next key mid-run changes nothing
# sent. A removal takes out only a key whose file carries the proxy's
# comment. These keep out a mistake, not an attacker: the folder is the
# person's alone, bound read-only inside the agent fence, so whatever can
# write it already runs as the person, with their gcloud login itself.
# That bind is agentic-harness.pi's fence, which binds each folder it lets
# through, pi-agent among them, read-only. Its runtime-binds test proves it,
# by the name kept whole on the next line for searching that repository,
# "can't remove, move or add anything there, so the proxy's socket stays the proxy's",
# which outlives a rename of the file, since no check here can follow it there.
# held() trusts the folder's socket on the same bind.
# The copy is never made in the temporary folder, which a fenced command
# can write, and so could swap a key of its own in before gcloud reads it.
#
# It runs the gcloud on the user manager's PATH, the one the person's
# login was made with. The fence binds ~/.local/bin, the Nix profile
# and ~/.config read-only, and the rest of that PATH is root's, so no
# fenced command can put a gcloud of its own ahead of the person's. A
# machine with no gcloud, or a gcloud with no login, registers nothing,
# and warns so in the journal, naming what would; neither fails the
# unit, as neither passes by trying again.
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
  1   gcloud couldn't register the key, or the proxy
      couldn't be asked for it
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
# up: the timer starts the unit again only once this one ends, and a
# register's wait and its three calls end well within the unit's four
# minutes. A removal is bounded tighter, since it holds the proxy's
# stop, and so a logout: its two calls take 14 seconds at most. A key
# it can't take out lapses on its own.
bound=60
[[ $mode == register ]] || bound=7
cloud() {
  timeout "$bound" gcloud compute os-login ssh-keys "$@" --quiet --verbosity=error
}

# The proxy lists its OS Login key under this comment alone, without
# the time it was made, which the file's comment adds (src/os_login.rs).
comment="pi-agent-proxy OS Login"
# The key, as kind and blob, that the proxy whose socket sits beside
# the file lists under its comment, when it lists exactly one and the
# file names the same; nothing otherwise. It fails 2 where the proxy
# couldn't be asked, which ssh-add says with 2 and timeout with 124,
# and 1 where it was asked and holds no such key, which an agent with
# no key at all says with 1. The proxy lists its own key only after
# each agent behind it answers or its 5 s patience with that agent ends,
# so asking waits that out: home.nix sets asking from the number of
# agents, and one agent's 5 s and a margin stand where it doesn't.
agent=${key%/*}/agent.sock
asking=${asking:-8}
held() {
  local kind blob rest listed line own='' asked=0
  read -r kind blob rest <"$key" || return 1
  listed=$(SSH_AUTH_SOCK=$agent timeout "$asking" ssh-add -L 2>/dev/null) || asked=$?
  ((asked == 0)) || return $((asked == 1 ? 1 : 2))
  while IFS= read -r line; do
    [[ $line == *" $comment" ]] || continue
    [[ -z $own ]] || return 1
    own=${line% "$comment"}
  done <<<"$listed"
  [[ -n $blob && $own == "$kind $blob" ]] || return 1
  echo "$own"
}
if ! command -v gcloud >/dev/null; then
  echo "<4>pi-agent-proxy-os-login: no gcloud on the user manager's PATH yet, so agents' OS" \
    "Login logins fail until one is; the desktop's start brings the session's PATH and tries" \
    "again, and jitsusama.pi.agentProxy.osLoginKnownHosts = null stops it where none is wanted"
  exit 0
fi
# The timer can fire as the proxy starts, a moment before it writes its
# key, while the file still holds the previous proxy's, and the path's
# event for the new key then joins this run rather than starting
# another; so a register waits a few seconds for the proxy to hold the
# key in the file, past the one ask that might take all of its own wait
# against an agent behind it. A proxy it still couldn't ask fails the
# unit, which tries again shortly, rather than wait out the quarter hour
# as if the proxy held no key.
if [[ $mode == register ]]; then
  own=
  why=1
  end=$((SECONDS + 5 + asking))
  while ((SECONDS < end)); do
    why=1
    if [[ -s $key ]]; then
      own=$(held) && break
      why=$?
      own=
    fi
    sleep 0.2 # The proxy writes its key in this while.
  done
  if [[ -z $own ]] && ((why == 2)); then
    echo "<3>pi-agent-proxy-os-login: the proxy at $agent couldn't be asked for its key;" \
      "trying again soon" >&2
    exit 1
  fi
  if [[ -z $own ]]; then
    echo "pi-agent-proxy-os-login: the proxy holds no key at $key, so it does nothing"
    exit 0
  fi
  line="$own $comment"
elif [[ ! -s $key ]]; then
  echo "pi-agent-proxy-os-login: no key at $key, so it does nothing"
  exit 0
else
  read -r kind blob rest <"$key" || rest=
  if [[ $rest != "$comment"* ]]; then
    echo "pi-agent-proxy-os-login: the key at $key isn't the proxy's, so it does nothing"
    exit 0
  fi
  # The line a register sent, under the bare comment, not the file's.
  line="$kind $blob $comment"
fi
# gcloud and ssh-keygen read the line from a file only this run writes,
# in the key's folder, which the fence binds read-only, so neither the
# key's file changing after the check nor a fenced command changes what's
# sent.
sent=$(mktemp -p "${key%/*}" .os-login-sent.XXXXXXXXXX)
trap 'rm -f "$sent"' EXIT
echo "$line" >"$sent"
# A gcloud that can't say fails as one with no login would: either way
# there's nothing to register with.
login=$(timeout "$bound" gcloud config get-value account --quiet 2>/dev/null) || login=
if [[ -z $login ]]; then
  echo "<4>pi-agent-proxy-os-login: gcloud has no login, so agents' OS Login logins fail;" \
    "gcloud auth login gives it one"
  exit 0
fi
# ssh-keygen says the bits, the fingerprint, then the comment.
fingerprint=$(ssh-keygen -lf "$sent")
fingerprint=${fingerprint#* }
fingerprint=${fingerprint%% *}

if [[ $mode == register ]]; then
  # update moves the end of a key already there; add puts in one that
  # isn't, as each new proxy's is, or one that lapsed.
  if cloud update --key-file="$sent" --ttl=12h >/dev/null 2>&1; then
    echo "pi-agent-proxy-os-login: $fingerprint renewed for 12 hours"
  elif cloud add --key-file="$sent" --ttl=12h >/dev/null; then
    echo "pi-agent-proxy-os-login: $fingerprint registered for 12 hours"
  else
    echo "<3>pi-agent-proxy-os-login: $fingerprint couldn't be registered; trying again soon," \
      "and gcloud auth login renews a lapsed login" >&2
    exit 1
  fi
else
  # The file stays, so no event of its going reaches the path unit as
  # the next proxy writes its own; that next key replaces it.
  if cloud remove --key-file="$sent" >/dev/null; then
    echo "pi-agent-proxy-os-login: $fingerprint removed"
  else
    echo "pi-agent-proxy-os-login: $fingerprint couldn't be removed; it lapses within 12 hours"
  fi
fi
