# shellcheck shell=bash
# Shows every-hardware-address-is-made-up.sh refuses a maker's address no
# one kept, with each second digit a maker's has and bounded by a colon at
# either end, and passes each kind of made-up one, one run on into a word
# or a further octet, and a kept one in upper case, saying what it says in
# the file given second. The kept one alone is refused, and named, once
# nothing is kept, and a tree with no address at all passes. The test keeps
# its own list, so the repository's may change without it.
# Usage: every-hardware-address-is-made-up-test.sh CHECK SAID
set -euo pipefail

check=$1
said=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/made-up" "$work/kept-only" "$work/none"
# Makers' addresses, one for each of 0, 4, 8 and c as the second digit,
# the second after an id's colon and the third and fifth before one, the
# fifth's beyond it running on into a word. They're written with dashes and
# given their colons here, since the check reads this file too and would
# refuse them as written.
printf '%s\n' '00-00-00-00-00-01' 'mac:04-00-00-00-00-01' 'device:48-00-00-00-00-01:map' \
  '0C-00-00-00-00-01' 'device:8c-00-00-00-00-01:cafe' | tr - : >"$work/made-up/copied.txt"
# Made-up ones of each kind, runs of seven and eight octets, one ending in
# a template, one at the tail of a word and one running on into a word
# (each the colon form of a maker's), seven octets ended by a hex word, and
# a kept one in upper case.
echo '02:00:00:00:a0:00 52:54:00:12:34:56 ff:ff:ff:ff:ff:ff 01:00:5e:00:00:fb' \
  '00:00:5E:00:53:01 00:00:00:00:00:00 00:00:00:00:00:00:01:01 ac:00:1b:66:aa:bb:cc' \
  'mac:00:00:5e:00:53:4{n} 00:1B:66:AA:BB:CC' \
  >"$work/made-up/made-up.txt"
printf '%s\n' 'id00-00-00-00-00-01' '00-00-00-00-00-01g' 'device:8c-00-00-00-00-01:fe' |
  tr - : >>"$work/made-up/made-up.txt"
echo '00:1b:66:aa:bb:cc' >"$work/kept"
echo 'A kept one: 00:1B:66:AA:BB:CC.' >"$work/kept-only/kept.txt"
echo 'Nothing here is an address.' >"$work/none/plain.txt"
if bash "$check" "$work/made-up" "$work/kept" 2>"$work/said"; then
  echo "It passed a maker's address no one kept." >&2
  exit 1
fi
if ! diff "$said" "$work/said"; then
  echo "It said other than the words above, as the lines it changed show." >&2
  exit 1
fi
if ! bash "$check" "$work/kept-only" "$work/kept"; then
  echo "It refused an address that was kept." >&2
  exit 1
fi
: >"$work/kept-none"
if bash "$check" "$work/kept-only" "$work/kept-none" 2>"$work/said"; then
  echo "It passed a maker's address with nothing kept." >&2
  exit 1
fi
if ! grep -qxF './kept.txt:1:00:1B:66:AA:BB:CC' "$work/said"; then
  echo "With nothing kept, it didn't name the kept address it refused:" >&2
  cat "$work/said" >&2
  exit 1
fi
if ! bash "$check" "$work/none" "$work/kept"; then
  echo "It refused a tree with no address in it." >&2
  exit 1
fi
