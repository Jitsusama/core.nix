# shellcheck shell=bash
# Runs device-check over files standing in for /proc, with a stand-in for
# notify-send, and checks it names a kernel that oopsed, locked up a
# processor, heard of a hardware error or found a bad page, and a task stuck
# in it, each once, while its other taints, a warning among them, say
# nothing, and neither does a taint that somehow clears, since the kernel
# never clears one. device-check comes from PATH.
# usage: kernel-trouble-is-named.sh
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
root=$work/root
kernel=$root/proc/sys/kernel
# The taint flags device-check reads, as the kernel numbers them.
warning=512
out_of_tree=4096
staging=1024
oops=128
lockup=16384
machine_check=16
bad_page=32

fail() {
  echo "$1"
  cat out
  exit 1
}
# Taints the stand-in kernel with the sum of the flags given.
tainted() {
  local sum=0 flag
  for flag in "$@"; do
    sum=$((sum + flag))
  done
  echo "$sum" > "$kernel/tainted"
}

mkdir -p "$kernel"
printf '#!%s\necho "$*" >> %s/notified\n' "$BASH" "$work" > notify
chmod +x notify
export DEVICE_CHECK_ROOT=$root DEVICE_CHECK_EXPECTED=$work/none
export DEVICE_CHECK_STATE=$work/state DEVICE_CHECK_NOTIFY=$work/notify
echo 120 > "$kernel/hung_task_timeout_secs"
echo 0 > "$kernel/hung_task_detect_count"

tainted "$warning" "$out_of_tree" "$staging"
device-check 2> out || fail "it failed on a kernel with harmless taints"
[ ! -s out ] || fail "it named a harmless taint"

tainted "$oops" "$warning" "$out_of_tree" "$staging"
device-check 2> out || fail "it failed on a kernel that oopsed"
grep -qxF "<4>The kernel hit a fault and may misbehave until the next boot" out \
  || fail "it didn't warn of the oops"
[ "$(wc -l < out)" = 1 ] || fail "it named more than the oops"
[ "$(grep -c '^-u critical -a Devices ' notified)" = 1 ] || fail "it didn't notify of the oops"

: > notified
device-check 2> out || fail "a second look failed"
[ ! -s out ] && [ ! -s notified ] || fail "it named the oops again"

echo 1 > "$kernel/hung_task_detect_count"
tainted "$oops" "$lockup" "$machine_check" "$bad_page"
device-check 2> out || fail "it failed on a kernel in worse trouble"
grep -qxF "<4>The kernel locked up a processor and may misbehave until the next boot" out \
  || fail "it didn't warn of the lockup"
grep -qxF "<4>The processor reported a hardware error" out \
  || fail "it didn't warn of the machine check"
grep -qxF "<4>The kernel found a damaged memory page" out || fail "it didn't warn of the bad page"
grep -qxF "<4>A task was stuck in the kernel for over 120 seconds" out \
  || fail "it didn't warn of the stuck task"
[ "$(wc -l < out)" = 4 ] || fail "it named the oops again, or more than what's new"

echo 2 > "$kernel/hung_task_detect_count"
device-check 2> out || fail "a look after a second stuck task failed"
[ ! -s out ] || fail "it named a second stuck task"

: > notified
tainted
echo 0 > "$kernel/hung_task_detect_count"
device-check 2> out || fail "a look at a clean kernel failed"
[ ! -s out ] && [ ! -s notified ] || fail "it said the kernel's trouble had cleared"
