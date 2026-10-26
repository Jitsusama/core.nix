# shellcheck shell=bash
# Traces an account's niri, its session's compositor, off the CPU
# (off-cpu.bt) until the unit running it is stopped, into a file in
# /run/niri-off-cpu named for the account, which only the account and root
# can read.
#
# usage: niri-off-cpu-as-root <account>

account=$1
id -u "$account" > /dev/null

# The oldest, since every `niri msg` the account runs is a niri too.
if ! pid=$(pgrep --oldest --uid "$account" --full '/bin/niri --session$'); then
  echo "no niri session runs as $account" >&2
  exit 1
fi

out=/run/niri-off-cpu/$account.txt
install -m 0600 -o "$account" /dev/null "$out"
# A line at a time, so the header BEGIN prints once every probe is in place
# reaches the file then, and the unit can wait for it (nixos.nix).
exec bpftrace -q -B line -o "$out" "$OFF_CPU" "$pid"
