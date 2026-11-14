# shellcheck shell=bash
# Runs the untrusted door's own daemon command, as its socket unit runs it,
# over a store of its own in the build's folder, and has a client that isn't
# trusted hand it impure-env entries, as dev and tec do with the credentials
# for private downloads. A fixed-output build must get the variable the
# daemon's unit names in untrusted-impure-env-names, through NIX_CONFIG as
# the module sets it, and no other, however the client asks; and no value may
# reach a warning or the daemon's log.
#
# Arguments: systemd-socket-activate, the nix a client runs, the build's
# expression (impure-env-build.nix), the unit's NIX_CONFIG, and the door's
# command, word by word.
set -euo pipefail

activate=$1
nix=$2
build=$3
named=$4
shift 4

here=$(pwd -P)
socket=$here/door.sock
store=$here/store
# From 2.35 a client opens the store folder it names before it connects, and
# the daemon would only make it on the first connection.
mkdir -p "$here/conf" "$store"
cat >"$here/conf/nix.conf" <<'EOF'
experimental-features = nix-command configurable-impure-env
sandbox = false
build-users-group =
substituters =
EOF

# The door's daemon gets only the environment named, as under systemd, so it
# keeps its store in the build's folder, reads only the nix.conf above and
# its unit's NIX_CONFIG, and has no NIX_UNLISTED of its own to hand a build.
# The clients never see that NIX_CONFIG.
HOME=$here NIX_CONF_DIR=$here/conf NIX_CONFIG=$named \
  NIX_REMOTE="local?store=$store&state=$here/state&log=$here/log" \
  "$activate" --accept --inetd -l "$socket" \
  -E HOME -E NIX_CONF_DIR -E NIX_CONFIG -E NIX_REMOTE "$@" \
  2>"$here/door.log" &
door=$!
trap 'kill "$door" 2>/dev/null || true' EXIT

for _ in $(seq 50); do
  [[ -S $socket ]] && break
  sleep 0.1
done

failed=0
# Builds one case through the door, with whatever the client is given after
# its four words, and says whether the build got what it expected.
case_() {
  local case=$1 vars=$2 expect=$3 says=$4
  shift 4
  if ! HOME=$here NIX_CONF_DIR=$here/conf "$nix" build --no-link --impure \
    --store "unix://$socket?store=$store" \
    --file "$build" --argstr sh "$BASH" --argstr case "$case" \
    --argstr vars "$vars" --argstr expect "$expect" "$@" 2>>"$here/client.log"; then
    printf 'FAIL: %s: %s\n' "$case" "$says" >&2
    failed=1
  fi
}

case_ named-by-nix-config NIX_LISTED named-by-nix-config:letmein \
  "a named variable set through NIX_CONFIG, as dev and tec set it, never reached the build" \
  --option extra-impure-env NIX_LISTED=letmein
case_ named-by-impure-env NIX_LISTED named-by-impure-env:letmein \
  "a named variable set as impure-env never reached the build" \
  --option impure-env NIX_LISTED=letmein
case_ unnamed NIX_UNLISTED unnamed: \
  "a variable the daemon doesn't name reached the build" \
  --option impure-env NIX_UNLISTED=sneaky
case_ named-beside-unnamed "NIX_LISTED NIX_UNLISTED" named-beside-unnamed:letmein: \
  "of a named and an unnamed variable set together, the build didn't get the one alone" \
  --option impure-env "NIX_LISTED=letmein NIX_UNLISTED=sneaky"
case_ widened NIX_UNLISTED widened: \
  "a client named its own variable in untrusted-impure-env-names and it reached the build" \
  --option untrusted-impure-env-names NIX_UNLISTED --option impure-env NIX_UNLISTED=sneaky
case_ malformed NIX_LISTED malformed: \
  "an entry that isn't name=value broke the build or set something" \
  --option impure-env not-a-pair-sneaky

if ! grep -q "impure-env variable 'NIX_UNLISTED'" "$here/client.log"; then
  printf 'FAIL: the client was never told its unnamed variable was dropped:\n' >&2
  cat "$here/client.log" >&2
  failed=1
fi
# A value may be a credential, so none reaches a warning or a log.
for log in "$here/client.log" "$here/door.log"; do
  if grep -q -e sneaky -e letmein "$log"; then
    printf 'FAIL: a value the client set reached %s\n' "${log##*/}" >&2
    failed=1
  fi
done

if ((failed)); then
  printf 'The client was told:\n' >&2
  cat "$here/client.log" >&2
  printf 'The door logged:\n' >&2
  cat "$here/door.log" >&2
  exit 1
fi
echo "An untrusted client hands a fixed-output build the variables the daemon names, and only them"
