# shellcheck shell=bash
# shellcheck disable=SC2310 # run and asked are steps, read in a condition to say which failed.
# Proves os-login.sh against a stand-in gcloud that writes down what it's
# asked: a new key added for twelve hours once renewing it fails, one
# already there renewed for twelve, a key removed with its file kept,
# failures said, every call bounded, and no gcloud, no gcloud login or
# no key passing over with a word.
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
# from /proc rather than by standing in for timeout.
cat >"$work/bin/gcloud" <<EOF
#!$BASH
echo "\$*" >>"$work/asked"
mapfile -d '' parent </proc/\$PPID/cmdline
echo "\${parent[0]##*/} \${parent[*]:1}" >>"$work/started-by"
if [[ \$1 == config ]]; then
  [[ -e "$work/no-login" ]] || echo someone@example.com
  exit 0
fi
[[ ! -e "$work/failing" ]] || exit 1
[[ \$4 != update || -e "$work/there" ]]
EOF
chmod +x "$work/bin/gcloud"
ssh-keygen -q -t ed25519 -N '' -C 'pi-agent-proxy OS Login 1' -f "$work/key"
key=$work/key.pub
fails=0
fail() {
  echo "os-login.sh $1" >&2
  fails=1
}
# Only the stand-in and the tools the script needs are on its PATH, so a
# real gcloud here never answers in the stand-in's place.
mkdir "$work/tools"
for tool in ssh-keygen timeout sleep; do
  found=$(command -v "$tool")
  ln -s "$found" "$work/tools/$tool"
done
run() {
  PATH=$work/bin:$work/tools "$register" "$@" >"$work/said" 2>&1
}
asked() {
  grep -qxF "compute os-login ssh-keys $1 --quiet --verbosity=error" "$work/asked"
}

run register "$key" || fail "failed registering a new key"
grep -q "registered for 12 hours" "$work/said" || fail "didn't say it registered a new key"
asked "add --key-file=$key --ttl=12h" || fail "didn't add a new key for twelve hours"

touch "$work/there"
: >"$work/asked"
run register "$key" || fail "failed renewing a key"
grep -q "renewed for 12 hours" "$work/said" || fail "didn't say it renewed a key"
asked "update --key-file=$key --ttl=12h" || fail "didn't renew the key for twelve hours"
grep -q "ssh-keys add" "$work/asked" && fail "added a key already there"

touch "$work/failing"
run register "$key" && fail "passed a key it couldn't register"
grep -q "couldn't be registered" "$work/said" || fail "didn't say registering failed"
run remove "$key" || fail "failed as a removal failed"
grep -q "lapses within 12 hours" "$work/said" || fail "didn't say a removal failed"
rm "$work/failing"

: >"$work/asked"
run remove "$key" || fail "failed removing a key"
asked "remove --key-file=$key" || fail "didn't remove the key"
[[ -e $key ]] || fail "took the key's file, an event the path unit would read as a new key"

run frobnicate "$key" && fail "took a mode that's neither register nor remove"

: >"$work/asked"
run register "$work/absent.pub" || fail "failed with no key"
[[ -s $work/asked ]] && fail "asked gcloud with no key"

touch "$work/no-login"
: >"$work/asked"
run register "$key" || fail "failed with no gcloud login, which trying again can't mend"
grep -q "^<4>.*gcloud auth login" "$work/said" || fail "didn't warn there's no gcloud login"
grep -q "ssh-keys" "$work/asked" && fail "asked for a key with no gcloud login"
rm "$work/no-login"

# Every call to the stand-in was started by timeout, bounded at a minute.
grep -qv "^timeout 60 gcloud " "$work/started-by" && fail "called gcloud unbounded"

rm "$work/bin/gcloud"
run register "$key" || fail "failed with no gcloud"
grep -q "^<4>.*no gcloud" "$work/said" || fail "didn't warn there's no gcloud"

exit "$fails"
