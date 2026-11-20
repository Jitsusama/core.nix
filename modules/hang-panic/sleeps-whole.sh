# shellcheck shell=bash # The check runs it with bash.
# Runs the sleep hook, the path named first, as systemd-sleep does, against
# a stand-in of busctl, and fails saying what it did wrong: the check
# hang-panic-sleeps-whole, in checks.nix. It works in the folder it's run in.
set -euo pipefail

hook=$1
export BUSCTL=$PWD/busctl KEPT=$PWD/kept
# systemd's word for its own setting.
setting=18446744073709551615

fail() {
  echo "$1" >&2
  exit 1
}

# The stand-in answers with the timeout in ./running and logs each change
# in ./changed.
cat >busctl <<EOF
#!$BASH
case "\$1" in
get-property) echo "t \$(cat "\$(dirname "\$0")/running")" ;;
set-property) echo "\$7" >>"\$(dirname "\$0")/changed" ;;
esac
EOF
chmod +x busctl

# systemd-sleep runs it with an empty PATH, the whole sleep's name as its
# second word, and the step it's taking in SYSTEMD_SLEEP_ACTION.
step() { PATH=/var/empty SYSTEMD_SLEEP_ACTION=$3 "$hook" "$1" "$2"; }

# What the hook told systemd, its changes in a line.
changes() {
  local lines=()
  [ ! -e changed ] || mapfile -t lines <changed
  echo "${lines[*]}"
}

echo 60000000 >running

for sleep in suspend:suspend suspend-then-hibernate:suspend; do
  step pre "${sleep%:*}" "${sleep#*:}"
  step post "${sleep%:*}" "${sleep#*:}"
  [ ! -e changed ] || fail "$sleep stopped the watchdog, which the driver stops by itself."
done

for sleep in hibernate:hibernate hybrid-sleep:hybrid-sleep suspend-then-hibernate:hibernate; do
  step pre "${sleep%:*}" "${sleep#*:}"
  told=$(changes)
  [ "$told" = 0 ] || fail "$sleep left the runtime watchdog running."
  step post "${sleep%:*}" "${sleep#*:}"
  told=$(changes)
  [ "$told" = "0 $setting" ] ||
    fail "The runtime watchdog wasn't handed back to systemd's setting after $sleep."
  [ ! -e kept ] || fail "What the hook kept outlived $sleep."
  rm changed
done

# A systemd from before SYSTEMD_SLEEP_ACTION names only the whole sleep.
PATH=/var/empty "$hook" pre hibernate
PATH=/var/empty "$hook" post hibernate
told=$(changes)
[ "$told" = "0 $setting" ] ||
  fail "A hibernation named only in the hook's words left the watchdog running."
rm changed

echo 0 >running
step pre hibernate hibernate
step post hibernate hibernate
[ ! -e changed ] || fail "A hibernation started a runtime watchdog that wasn't running."
