# shellcheck shell=bash
# Runs the seal against a made-up BlueZ state, with a stand-in systemd-creds
# that writes what it's given and says how it was asked to seal it, and a
# stand-in sync, since a build's sandbox can't sync a file system. Given the
# seal's path.
set -euo pipefail

seal=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export BLUEZ_STATE=$work/bluetooth ESP=$work/esp STATE_DIRECTORY=$work/state POLICY=$work/policy.json
mkdir -p "$STATE_DIRECTORY" "$work/bin"
credential=$ESP/loader/credentials/bluetooth-pairing.cred

cat >"$work/bin/systemd-creds" <<EOF
#!$BASH
printf '%s\n' "\$*" >>"$work/asked"
cp "\${@: -2:1}" "\${@: -1}"
EOF
printf '#!%s\n' "$BASH" >"$work/bin/sync"
chmod +x "$work/bin/systemd-creds" "$work/bin/sync"
export PATH=$work/bin:$PATH

fail() {
  echo "$1"
  exit 1
}

adapter=$BLUEZ_STATE/AA:BB:CC:DD:EE:FF
paired=$adapter/11:22:33:44:55:66
mkdir -p "$adapter/cache"
printf '[General]\nDiscoverable=false\n' >"$adapter/settings"
printf '[ServiceRecords]\n0x10000=keyboard\n' >"$adapter/cache/11:22:33:44:55:66"
printf '[ServiceRecords]\n0x10000=a stranger\n' >"$adapter/cache/99:88:77:66:55:44"

# Nothing paired leaves nothing on the ESP, and seals nothing.
"$BASH" "$seal"
[ ! -e "$credential" ] || fail "it sealed with nothing paired"
[ ! -e "$work/asked" ] || fail "it asked the TPM with nothing paired"

# A paired keyboard is sealed with its keys and its service records, bound
# to the policy, and a device only seen in a scan stays out.
mkdir -p "$paired"
printf '[General]\nName=Keyboard\n[LinkKey]\nKey=00112233445566778899AABBCCDDEEFF\n' >"$paired/info"
"$BASH" "$seal"
[ -e "$credential" ] || fail "it sealed nothing with a keyboard paired"
grep -q -- "--name=bluetooth-pairing --with-key=tpm2 --tpm2-pcrlock=$POLICY" "$work/asked" ||
  fail "it didn't seal to the TPM under the policy: $(cat "$work/asked")"
listing=$(tar --list --file="$credential" | sort | tr '\n' ' ')
expected="AA:BB:CC:DD:EE:FF/11:22:33:44:55:66/info AA:BB:CC:DD:EE:FF/cache/11:22:33:44:55:66 "
expected+="AA:BB:CC:DD:EE:FF/settings "
[ "$listing" = "$expected" ] || fail "it sealed $listing, not $expected"

# Unchanged pairings aren't sealed again, so the ESP isn't written.
"$BASH" "$seal"
[ "$(wc -l <"$work/asked")" = 1 ] || fail "it sealed unchanged pairings again"

# Without measured boot it binds to Secure Boot's state alone.
POLICY="" && printf 'Name=Renamed Keyboard\n' >>"$paired/info"
"$BASH" "$seal"
tail -n 1 "$work/asked" | grep -q -- "--tpm2-pcrs=7" || fail "it didn't bind to PCR 7 without a policy"

# Once unpaired, the sealed keys leave the ESP.
rm -r "$paired"
"$BASH" "$seal"
[ ! -e "$credential" ] || fail "the sealed keys stayed on the ESP once unpaired"
echo "the seal keeps what a keyboard needs, and only that"
