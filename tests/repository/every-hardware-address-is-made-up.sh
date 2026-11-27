# shellcheck shell=bash
# No hardware address in the repository could be a real device's, since a
# real one, beside the network or the place a test names, says where a
# machine has been. An address is made up when it says so in its first
# octet: locally administered (02 there, as tests use, or QEMU's 52:54:00)
# or a group's (ff:ff:... or 01:00:5e:...), neither of which a maker gives
# a device. RFC 7042's documentation block, 00:00:5e:00:53:xx, and all
# zeroes name no device either.
#
# Every other address is one a maker could have given, so it passes only
# when it's on the list of those kept on purpose, given as the second
# argument, one address a line: a vendor's real prefix that a vendor's name
# is read from, under a device part plainly made up. It reads addresses
# written with colons, six octets alone, so a longer run of octets, such
# as a key, isn't taken for one, and it prints each address it refuses.
set -euo pipefail

kept=$(realpath "$2")
cd "$1"

# The second hex digit of the first octet holds both bits: 0, 4, 8 or c
# there is a maker's address for one device. The list of kept addresses
# lives in the checks that give it, so that file isn't read.
maker='(?<![0-9A-Fa-f:])[0-9A-Fa-f][048Cc](:[0-9A-Fa-f]{2}){5}(?![0-9A-Fa-f:])'
status=0
found=$(rg --pcre2 --hidden --only-matching --line-number --no-heading --with-filename \
  --glob '!.git' --glob '!flake.lock' --glob '!tests/repository/checks.nix' \
  "$maker" .) || status=$?
if [ "$status" -eq 1 ]; then
  exit 0
elif [ "$status" -ne 0 ]; then
  echo "The search for hardware addresses failed, status $status." >&2
  exit "$status"
fi

# The kept list is told by its name rather than by NR == FNR, which an
# empty list would turn onto the addresses found.
refused=$(awk -F: '
  FILENAME == ARGV[1] { kept[tolower($0)] = 1; next }
  {
    address = tolower($(NF - 5) ":" $(NF - 4) ":" $(NF - 3) ":" $(NF - 2) ":" $(NF - 1) ":" $NF)
    if (address ~ /^00:00:5e:00:53:/ || address == "00:00:00:00:00:00" || address in kept) next
    print
  }
' "$kept" - <<<"$found")
if [ -n "$refused" ]; then
  printf '%s\n' "$refused" >&2
  cat >&2 <<'EOF'
The addresses above could be a real device's. Write a made-up one, as
02:00:00:00:a0:00, or RFC 7042's 00:00:5e:00:53:01; an address that needs
a vendor's real prefix is kept on purpose in tests/repository/checks.nix.
EOF
  exit 1
fi
