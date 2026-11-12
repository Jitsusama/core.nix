# shellcheck shell=bash
# Whether anyone's launcher is in use, as update-to-main asks before it
# switches (nixos.nix): open, or closed under 30 s ago. The shell writes
# "open" or "closed" in its quickshell-launcher file only as it changes,
# so the file's time dates the close, and someone who just pressed Return
# in it is still at it. Exits 0 while one is, and 1 while none is.
now=$(date +%s)
for mark in /run/user/*/quickshell-launcher; do
  test -e "${mark}" || continue
  grep -qx open "${mark}" 2>/dev/null && exit 0
  written=$(stat -c %Y "${mark}" 2>/dev/null) || continue
  test $((now - written)) -lt 30 && exit 0
done
exit 1
