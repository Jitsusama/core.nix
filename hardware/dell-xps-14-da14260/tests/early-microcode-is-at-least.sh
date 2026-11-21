# shellcheck shell=bash
# Checks the early microcode image a machine's initrd is set to start with
# gives a processor a revision at least as new as the one named, so the
# processor runs it from the first instruction of the kernel. The kernel
# loads an update only when the processor's platform bit, its
# processor_flags, is in the update's pf_mask, so only those count.
# usage: early-microcode-is-at-least.sh <image> <signature> <platform bit> <revision>
# e.g. early-microcode-is-at-least.sh intel-ucode.img 0x000c06c2 0x80 0x011d
# It needs bsdtar and iucode_tool on PATH.
set -euo pipefail

image=$1 signature=$2 platform=$3 wanted=$4
work=$(mktemp -d)
bsdtar -xf "$image" -C "$work"
bundle="$work/kernel/x86/microcode/GenuineIntel.bin"
if [[ ! -s $bundle ]]; then
  echo "The image holds no Intel microcode, so the processor runs its own." >&2
  exit 1
fi

# iucode_tool names every signature an update covers, its extended ones
# included, one a line, as "sig 0x000c06c2, pf_mask 0x94, 2026-07-07, rev 0x011d".
listed=$(iucode_tool -L -t b "$bundle")
newest=0
while read -r mask revision; do
  if ((mask & platform && revision > newest)); then newest=$((revision)); fi
done < <(grep -o "sig $signature, pf_mask 0x[0-9a-f]*, [0-9-]*, rev 0x[0-9a-f]*" <<<"$listed" |
  sed 's/.*pf_mask \(0x[0-9a-f]*\), .*rev \(0x[0-9a-f]*\)/\1 \2/' || true)

if ((newest == 0)); then
  echo "The image has no microcode for $signature on platform $platform." >&2
  exit 1
fi
if ((newest < wanted)); then
  printf 'The image gives %s revision 0x%x, older than %s.\n' "$signature" "$newest" "$wanted" >&2
  exit 1
fi
printf 'The image gives %s revision 0x%x.\n' "$signature" "$newest"
