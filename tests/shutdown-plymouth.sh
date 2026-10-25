# shellcheck shell=bash
# Stands in for Plymouth's client in tests/shutdown.nix: what a reboot tells Plymouth, given
# as its second argument, goes to the kernel's log, where tests/shutdown-vm-test.py reads it
# from the console.
echo "<3>told plymouth $2" > /dev/kmsg
