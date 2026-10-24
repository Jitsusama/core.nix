# shellcheck shell=bash
# Runs firmware-settings over files standing in for the kernel's
# firmware-attributes class, and checks it names every setting that differs
# from the declared ones, writes them with the admin password from its
# input, wipes that password from the kernel whether the writes worked or
# not, and refuses a value it can't read rather than calling it changed.
# The unreadable and unwritable settings hold only for a user other than
# root, as a build's is.
# firmware-settings comes from PATH.
# usage: the-firmware-takes-what-is-declared.sh
set -euo pipefail

work=$(mktemp -d)
trap 'chmod -R u+rw "$work"; rm -rf "$work"' EXIT
cd "$work"
fw=$work/fw/vendor
admin=$fw/authentication/Admin

# A setting the firmware holds, at the value given.
setting() {
  mkdir -p "$fw/attributes/$1"
  echo "$2" > "$fw/attributes/$1/current_value"
}
value() {
  cat "$fw/attributes/$1/current_value"
}
password() {
  tr -d '\n' < "$admin/current_password"
}
fail() {
  echo "$1"
  cat out
  exit 1
}
# Runs firmware-settings check, and says what it exited with.
checked() {
  local status=0
  firmware-settings check > out 2>&1 || status=$?
  echo "$status"
}

mkdir -p "$admin" "$fw/attributes"
echo 1 > "$admin/is_enabled"
: > "$admin/current_password"
echo 0 > "$fw/attributes/pending_reboot"
setting Mode Adaptive
setting Start 50
setting Stop 90
printf 'Mode=Custom\nStart=75\nStop=80\n' > declared
export FIRMWARE_ATTRIBUTES=$work/fw FIRMWARE_SETTINGS=$work/declared

[ "$(checked)" != 0 ] || fail "check passed with settings to change"
grep -qF "Start is 50, declared 75" out || fail "check didn't name what differs"

echo secret | firmware-settings apply > out 2>&1 || fail "apply failed"
[ "$(value Mode)/$(value Start)/$(value Stop)" = Custom/75/80 ] \
  || fail "apply left $(value Mode)/$(value Start)/$(value Stop)"
[ -z "$(password)" ] || fail "apply left the password in the kernel"
[ "$(checked)" = 0 ] || fail "check failed after apply"

setting Locked old
chmod a-w "$fw/attributes/Locked/current_value"
echo Locked=new >> declared
echo secret | firmware-settings apply > out 2>&1 && fail "apply passed a setting it couldn't write"
grep -qF "Locked is old, declared new" out || fail "apply didn't name what it couldn't write"
[ -z "$(password)" ] || fail "a failed apply left the password in the kernel"

echo 0 > "$admin/is_enabled"
printf 'Mode=Standard\n' > declared
firmware-settings apply < /dev/null > out 2>&1 || fail "apply asked for a password none is set for"
[ "$(value Mode)" = Standard ] || fail "apply without a password didn't write"

printf 'Missing=x\n' > declared
status=$(checked)
[ "$status" = 2 ] || fail "check of a setting the firmware lacks exited $status"
grep -qF "no firmware setting Missing" out || fail "check didn't name the missing setting"

chmod a-r "$fw/attributes/Mode/current_value"
printf 'Mode=Standard\n' > declared
status=$(checked)
[ "$status" = 77 ] || fail "check of a setting it can't read exited $status"
grep -qF "can't read Mode; run it as root: run0 firmware-settings check" out \
  || fail "check didn't say to run it as root"
if grep -qF "Mode is" out; then
  fail "check named a setting it couldn't read as differing"
fi
