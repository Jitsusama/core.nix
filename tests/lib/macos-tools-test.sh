#!/usr/bin/env bash
# The check that macosTools (macos-tools.nix) runs a script with macOS's
# own tools, as a Mac's terminal runs them: tr, sed and find are /usr/bin's,
# a tool only GNU's coreutils has isn't found at all, and tr under the Mac's
# UTF-8 locale refuses a byte that isn't UTF-8 unless LC_ALL=C has it read
# bytes. GNU's tr, first in a darwin build otherwise, reads the byte either
# way, so a script that leaves LC_ALL=C out passes there and fails on the
# Mac, and here.
set -euo pipefail

for tool in tr sed find; do
  found=$(command -v "${tool}" || true)
  if [[ "${found}" != "/usr/bin/${tool}" ]]; then
    echo "${tool} is ${found:-not found}, not macOS's own /usr/bin/${tool}" >&2
    exit 1
  fi
done

if gnu=$(command -v tac); then
  echo "tac, which macOS hasn't, is ${gnu}, so a tool macOS lacks is still found" >&2
  exit 1
fi

if said=$(printf 'a\377b\n' | tr '[:lower:]' '[:upper:]' 2>&1 >/dev/null); then
  echo "tr read a byte that isn't UTF-8 under ${LANG:-no locale}, as GNU's tr does" >&2
  exit 1
fi
if [[ "${said}" != *"Illegal byte sequence"* ]]; then
  echo "tr refused a byte that isn't UTF-8 by saying '${said}', not 'Illegal byte sequence'" >&2
  exit 1
fi

read=$(printf 'a\377b\n' | LC_ALL=C tr '[:lower:]' '[:upper:]' | od -An -tx1 | tr -d ' \n')
if [[ "${read}" != "41ff420a" ]]; then
  echo "tr under LC_ALL=C read the bytes 61 ff 62 0a as ${read}, not 41ff420a" >&2
  exit 1
fi
