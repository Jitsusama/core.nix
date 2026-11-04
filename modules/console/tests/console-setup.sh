# shellcheck shell=bash
# Runs the consoles' setup (modules/console/setup.sh, as a machine builds
# it) with stand-ins for systemd-vconsole-setup and kbdinfo, and checks
# what it does: a setup that works is done; one that fails on a console a
# compositor has since taken for graphics is done too, saying so; and one
# that fails on a console still showing text fails with the setup's own
# status.
# usage: console-setup.sh <console-setup>
set -euo pipefail

work=$(mktemp -d)
export WORK=$work
mkdir "$work/bin"

# The setup exits with the status in $WORK/status, and kbdinfo says the
# mode in $WORK/mode of the console it's asked about, which is recorded.
{
  printf '#!%s\n' "$BASH"
  echo 'exit "$(< "$WORK/status")"'
} > "$work/setup"
{
  printf '#!%s\n' "$BASH"
  echo 'echo "$2" > "$WORK/asked"; cat "$WORK/mode"'
} > "$work/bin/kbdinfo"
chmod +x "$work/setup" "$work/bin/kbdinfo"

# The setup with the stand-ins in place of what it names.
sed -e "s|^export PATH=\"|export PATH=\"$work/bin:|" -e "s|^setup=.*|setup=$work/setup|" "$1" \
  > "$work/console-setup"

fails() {
  echo "the consoles' setup: $1" >&2
  exit 1
}

# Runs it as udev's restart does, with no console named, and keeps its
# status and what it said.
run() {
  echo "$1" > "$work/status"
  echo "$2" > "$work/mode"
  status=0
  "$BASH" "$work/console-setup" > "$work/said" 2>&1 || status=$?
}

run 0 text
[[ $status == 0 ]] || fails "a setup that worked ended $status"

run 1 graphics
[[ $status == 0 ]] || fails "a setup that failed on a console taken for graphics ended $status"
[[ $(< "$work/asked") == /dev/tty1 ]] || fails "it asked about $(< "$work/asked"), not /dev/tty1"
grep -q 'graphics' "$work/said" || fails "it didn't say the console was taken: $(< "$work/said")"

run 1 text
[[ $status == 1 ]] || fails "a setup that failed on a console showing text ended $status, not 1"
