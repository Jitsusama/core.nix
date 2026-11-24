# shellcheck shell=bash
# Proves os-login.sh against a stand-in gcloud that writes down what it's
# asked: a new key added for twelve hours once renewing it fails, one
# already there renewed, a key removed and its file gone with it, and no
# gcloud or no key passing quietly over.
#
#   os-login-test.sh PATH-TO-pi-agent-proxy-os-login

set -euo pipefail
register=$1
work=$(mktemp -d)
mkdir "$work/bin"
cat >"$work/bin/gcloud" <<EOF
#!$BASH
echo "\$*" >>"$work/asked"
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

PATH=$work/bin:$PATH "$register" register "$key" >"$work/said"
grep -q "registered for 12 hours" "$work/said" || fail "didn't say it registered a new key"
grep -qx "compute os-login ssh-keys add --key-file=$key --ttl=12h --quiet --verbosity=error" \
  "$work/asked" || fail "didn't add a new key for twelve hours"

touch "$work/there"
: >"$work/asked"
PATH=$work/bin:$PATH "$register" register "$key" >"$work/said"
grep -q "renewed for 12 hours" "$work/said" || fail "didn't say it renewed a key"
grep -q "ssh-keys add" "$work/asked" && fail "added a key already there"

: >"$work/asked"
PATH=$work/bin:$PATH "$register" remove "$key" >"$work/said"
grep -qx "compute os-login ssh-keys remove --key-file=$key --quiet --verbosity=error" \
  "$work/asked" || fail "didn't remove the key"
[[ -e $key ]] && fail "left the key's file once it was removed"

: >"$work/asked"
PATH=$work/bin:$PATH "$register" register "$key" >"$work/said" || fail "failed with no key"
[[ -s $work/asked ]] && fail "asked gcloud with no key"

rm "$work/bin/gcloud"
PATH=$work/bin:$PATH "$register" register "$work/key.pub" >"$work/said" ||
  fail "failed with no gcloud"
grep -q "no gcloud" "$work/said" || fail "didn't say there's no gcloud"

exit "$fails"
