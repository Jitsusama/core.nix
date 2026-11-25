# shellcheck shell=bash
# shellcheck disable=SC2310 # run and asked are steps, read in a condition to say which failed.
# Proves os-login.sh against a stand-in gcloud that writes down what it's
# asked: a new key added for twelve hours once renewing it fails, one
# already there renewed for twelve, a key removed with its file kept,
# failures said, every call bounded, and no gcloud, no gcloud login, a
# gcloud that can't say, no key or a key the proxy doesn't hold passing
# over with a word; a key the proxy comes to hold within its wait is
# registered, another key it holds never is but leaves the proxy's own
# registered beside it, what gcloud reads is the proxy's own line and
# never the file's, copied beside the key and never into the temporary
# folder, a removal sends the line a register did, a proxy that can't be
# asked or never answers fails to be tried again, and a key that isn't
# the proxy's is never removed.
#
#   os-login-test.sh PATH-TO-pi-agent-proxy-os-login

set -euo pipefail
register=$1
work=$(mktemp -d)
mkdir "$work/bin"
# The stand-in writes down what it's asked and what started it, names
# its login unless told it has none, fails an update while no key is
# there, as gcloud's own update.py does, and fails everything while told
# to fail. The script's own PATH comes first, so what started it is read
# from /proc rather than by standing in for timeout. It writes down the
# key it's handed, as gcloud reads it.
cat >"$work/bin/gcloud" <<EOF
#!$BASH
echo "\$*" >>"$work/asked"
for arg; do
  [[ \$arg == --key-file=* ]] || continue
  cat "\${arg#--key-file=}" >>"$work/sent"
  echo "\${arg#--key-file=}" >>"$work/copies"
done
mapfile -d '' parent </proc/\$PPID/cmdline
echo "\${parent[0]##*/} \${parent[*]:1}" >>"$work/started-by"
if [[ \$1 == config ]]; then
  [[ ! -e "$work/config-fails" ]] || exit 1
  [[ -e "$work/no-login" ]] || echo someone@example.com
  exit 0
fi
[[ ! -e "$work/failing" ]] || exit 1
[[ \$4 != update || -e "$work/there" ]]
EOF
chmod +x "$work/bin/gcloud"
# The proxy stands in as a real ssh-agent at its socket beside the key,
# since the script's own PATH puts the real ssh-add ahead of any stand-in;
# it holds the keys the test gives it, as the proxy holds the one it made.
ssh-agent -D -a "$work/agent.sock" >/dev/null 2>&1 &
agent=$!
trap 'kill -CONT "$agent"; kill "$agent"' EXIT
until [[ -S $work/agent.sock ]]; do
  kill -0 "$agent" || exit 1
  sleep 0.1
done
hold() {
  SSH_AUTH_SOCK=$work/agent.sock ssh-add -q "$1" 2>/dev/null
}
hold_none() {
  SSH_AUTH_SOCK=$work/agent.sock ssh-add -qD 2>/dev/null
}
# The proxy lists its key under its comment alone, and its file adds the
# time the key was made.
make_key() {
  local kind blob
  ssh-keygen -q -t ed25519 -N '' -C "$2" -f "$work/$1"
  read -r kind blob _ <"$work/$1.pub"
  echo "$kind $blob $2${3:+ $3}" >"$work/$1.pub"
}
make_key key 'pi-agent-proxy OS Login' 1
key=$work/key.pub
read -r _ blob _ <"$key"
hold "$work/key"
fails=0
fail() {
  echo "os-login.sh $1" >&2
  fails=1
}
# Only the stand-in and the tools the script needs are on its PATH, so a
# real gcloud here never answers in the stand-in's place.
mkdir "$work/tools"
for tool in ssh-keygen ssh-add timeout sleep cp cat mktemp rm; do
  found=$(command -v "$tool")
  ln -s "$found" "$work/tools/$tool"
done
# The temporary folder a fenced command could write, which no copy may
# be made in.
mkdir "$work/shared"
run() {
  TMPDIR=$work/shared PATH=$work/bin:$work/tools "$register" "$@" >"$work/said" 2>&1
}
# A call asked of gcloud, as a pattern, since a register hands it a file
# of its own.
asked() {
  grep -qxE "compute os-login ssh-keys $1 --quiet --verbosity=error" "$work/asked"
}

run register "$key" || fail "failed registering a new key"
grep -q "registered for 12 hours" "$work/said" || fail "didn't say it registered a new key"
asked "add --key-file=[^ ]+ --ttl=12h" || fail "didn't add a new key for twelve hours"
grep -qxF "ssh-ed25519 $blob pi-agent-proxy OS Login" "$work/sent" ||
  fail "didn't hand gcloud the proxy's own line"
touch "$work/there"
: >"$work/asked"
run register "$key" || fail "failed renewing a key"
grep -q "renewed for 12 hours" "$work/said" || fail "didn't say it renewed a key"
asked "update --key-file=[^ ]+ --ttl=12h" || fail "didn't renew the key for twelve hours"
grep -q "ssh-keys add" "$work/asked" && fail "added a key already there"
grep -qxF "$(<"$key")" "$work/sent" && fail "handed gcloud the key's file, not the proxy's line"

touch "$work/failing"
run register "$key" && fail "passed a key it couldn't register"
grep -q "couldn't be registered" "$work/said" || fail "didn't say registering failed"
run remove "$key" || fail "failed as a removal failed"
grep -q "lapses within 12 hours" "$work/said" || fail "didn't say a removal failed"
rm "$work/failing"

: >"$work/asked"
: >"$work/sent"
run remove "$key" || fail "failed removing a key"
asked "remove --key-file=[^ ]+" || fail "didn't remove the key"
[[ $(<"$work/sent") == "ssh-ed25519 $blob pi-agent-proxy OS Login" ]] ||
  fail "sent a removal a line other than the one a register sends"
[[ -e $key ]] || fail "took the key's file, an event the path unit would read as a new key"

status=0
run frobnicate "$key" || status=$?
[[ $status -eq 64 ]] || fail "took a mode that's neither register nor remove, with $status"

run --help || fail "failed saying its help"
grep -q "^EXIT" "$work/said" || fail "said no help"

# A previous proxy's key, left in the file, is one the proxy now running
# doesn't hold.
make_key other 'pi-agent-proxy OS Login' 2
hold_none
hold "$work/other"
: >"$work/asked"
run register "$key" || fail "failed with a key the proxy doesn't hold"
grep -q "holds no key" "$work/said" || fail "didn't say the proxy holds no such key"
grep -q "ssh-keys" "$work/asked" && fail "registered a key the proxy doesn't hold"

# The proxy comes to hold the key while the register waits.
hold_none
: >"$work/asked"
(
  sleep 1
  hold "$work/key"
) &
holding=$!
run register "$key" || fail "failed with a key the proxy came to hold"
wait "$holding"
asked "update --key-file=[^ ]+ --ttl=12h" || fail "didn't wait for the proxy to hold its key"

# A previous proxy's key answered after an agent behind held the ask
# up past the first few seconds, as a proxy restarting may, still leaves
# time to ask again once the proxy holds the new one.
hold_none
hold "$work/other"
kill -STOP "$agent"
: >"$work/asked"
(
  sleep 6
  kill -CONT "$agent"
  # The ask held up is answered first, as the agent takes them in turn.
  hold_none
  hold "$work/key"
) &
holding=$!
run register "$key" || fail "failed with a slow answer naming the previous key"
wait "$holding"
asked "update --key-file=[^ ]+ --ttl=12h" ||
  fail "gave up on the key once one slow answer named the previous one"

# Another key the proxy holds, the builders' among them, is never
# registered, even with a file naming it beside the proxy's socket.
make_key builders 'nix-offload builders'
hold_none
hold "$work/builders"
: >"$work/asked"
run register "$work/builders.pub" || fail "failed with a key that isn't the proxy's own"
grep -q "ssh-keys" "$work/asked" && fail "registered a key the proxy holds that isn't its own"

# Beside it, the proxy's own is still registered, the builders' listed
# first, as the proxy lists them.
hold "$work/key"
: >"$work/asked"
run register "$key" || fail "failed with the builders' key held beside the proxy's"
asked "update --key-file=[^ ]+ --ttl=12h" ||
  fail "didn't register the proxy's key with the builders' held beside it"

# Nor is a key held beside another under the proxy's comment, as no
# proxy lists; the other comes first, since the agent lists in order.
make_key twin 'pi-agent-proxy OS Login' 3
hold_none
hold "$work/twin"
hold "$work/key"
: >"$work/asked"
run register "$key" || fail "failed with two keys under the proxy's comment"
grep -q "ssh-keys" "$work/asked" && fail "registered one of two keys under the proxy's comment"
hold_none
hold "$work/key"

# A proxy that can't be asked fails the unit, to be tried again, rather
# than pass over as one holding no key.
mkdir "$work/unasked"
cp "$key" "$work/unasked/key.pub"
: >"$work/asked"
run register "$work/unasked/key.pub" && fail "passed with a proxy it couldn't ask"
grep -q "^<3>.*couldn't be asked" "$work/said" || fail "didn't say the proxy couldn't be asked"
grep -q "ssh-keys" "$work/asked" && fail "asked gcloud with a proxy it couldn't ask"

# Nor does one that takes every ask but never answers, as a proxy
# waiting on a stalled agent behind it might, pass as holding no key.
kill -STOP "$agent"
: >"$work/asked"
run register "$key" && fail "passed with a proxy that never answered"
kill -CONT "$agent"
grep -q "^<3>.*couldn't be asked" "$work/said" || fail "didn't say a silent proxy couldn't be asked"
grep -q "ssh-keys" "$work/asked" && fail "asked gcloud with a proxy that never answered"

# A removal never takes out a key whose file isn't the proxy's, such as
# the person's own.
: >"$work/asked"
run remove "$work/builders.pub" || fail "failed with a key that isn't the proxy's to remove"
grep -q "ssh-keys" "$work/asked" && fail "removed a key that isn't the proxy's"

: >"$work/asked"
run register "$work/absent.pub" || fail "failed with no key"
[[ -s $work/asked ]] && fail "asked gcloud with no key"

touch "$work/no-login"
: >"$work/asked"
run register "$key" || fail "failed with no gcloud login, which trying again can't mend"
grep -q "^<4>.*gcloud auth login" "$work/said" || fail "didn't warn there's no gcloud login"
grep -q "ssh-keys" "$work/asked" && fail "asked for a key with no gcloud login"
rm "$work/no-login"

touch "$work/config-fails"
: >"$work/asked"
run register "$key" || fail "failed as gcloud couldn't say its login"
grep -q "^<4>.*gcloud auth login" "$work/said" || fail "didn't warn gcloud couldn't say its login"
grep -q "ssh-keys" "$work/asked" && fail "asked for a key with gcloud unable to say its login"
rm "$work/config-fails"

# Every call to the stand-in was started by timeout: a register's at a
# minute, a removal's two, which hold the proxy's stop, at 7 s each.
grep -Eqv "^timeout (60|7) gcloud " "$work/started-by" && fail "called gcloud unbounded"
grep -q "^timeout 7 gcloud compute os-login ssh-keys remove " "$work/started-by" ||
  fail "didn't bound a removal tighter"
grep -q "^timeout 7 gcloud compute os-login ssh-keys \(add\|update\) " "$work/started-by" &&
  fail "bounded a registration as tightly as a removal"

rm "$work/bin/gcloud"
run register "$key" || fail "failed with no gcloud"
grep -q "^<4>.*no gcloud" "$work/said" || fail "didn't warn there's no gcloud"

# Every run's copy, the removals' and the waiting cases' too, since the
# script's trap would remove a stray one before the folder is listed.
grep -qvE "^$work/\.os-login-sent\.[^/]+$" "$work/copies" &&
  fail "copied the proxy's line outside the key's folder"
left=$(ls -A "$work/shared")
[[ -z $left ]] || fail "made a file in the temporary folder"
compgen -G "$work/.os-login-sent.*" >/dev/null && fail "left a copy of the line behind"

exit "$fails"
