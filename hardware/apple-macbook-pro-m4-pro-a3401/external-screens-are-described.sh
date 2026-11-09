# shellcheck shell=bash
# Checks the device tree the 14-inch M4 Pro's kernel builds describes both
# external screens: the HDMI port's display coprocessor and the one routed
# to the USB-C ports, each waiting, disabled, for the boot loader to fill in
# its firmware's memory; the crossbars that route a coprocessor to a port;
# and the HDMI converter's hot-plug pin. Aurora's board file takes all of
# it from the 16-inch's, and every value was checked against a J614s's own
# ADT once; this holds that a new commit keeps it there. The tree is
# compiled as the kernel's build compiles it, from the source's own files
# with the machine's patches on them, with nothing else of the kernel
# unpacked or built.
# usage: external-screens-are-described.sh <kernel source tarball> [patch...]
set -euo pipefail

source=$(mktemp -d)
tar xf "$1" -C "$source" --strip-components=1 --wildcards \
  '*/arch/arm64/boot/dts/apple/*' '*/include/dt-bindings/*' \
  '*/scripts/dtc/include-prefixes/*'
shift
# The machine's own patches, as far as they reach the device trees.
for patch in "$@"; do
  reach=$(filterdiff -p1 -i 'arch/arm64/boot/dts/*' -i 'include/dt-bindings/*' "$patch")
  if [ -n "$reach" ]; then
    patch -d "$source" -p1 --forward --batch --silent <<<"$reach"
  fi
done

dts=$source/arch/arm64/boot/dts
blob=$(mktemp)

cpp -nostdinc -undef -D__DTS__ -x assembler-with-cpp \
  -I "$dts/apple" -I "$dts" -I "$source/include" \
  -I "$source/scripts/dtc/include-prefixes" \
  "$dts/apple/t6040-j614s.dts" |
  dtc -q -I dts -O dtb -o "$blob" -

failed=
# Says what a node's property should be and isn't, if it isn't.
expect() {
  local node=$1 property=$2 want=$3 got
  got=$(fdtget -t s "$blob" "$node" "$property" 2>/dev/null | tr '\0' ' ') || got="(none)"
  if [ "$got" != "$want" ]; then
    echo "$node's $property is \"$got\", where the external screens need \"$want\"."
    failed=1
  fi
}
# Says a node or a property that should be there isn't.
present() {
  local node=$1 property=${2:-}
  if [ -n "$property" ]; then
    fdtget "$blob" "$node" "$property" >/dev/null 2>&1 && return
    echo "$node has no $property, which the external screens need."
  else
    fdtget -l "$blob" "$node" >/dev/null 2>&1 && return
    echo "$node isn't in the tree, and the external screens need it."
  fi
  failed=1
}

expect / compatible "apple,j614s apple,t6040 apple,arm-platform"

# HDMI's coprocessor, then the USB-C ports'.
for dcp in /dcp@3a2e00000 /dcp@602e00000; do
  expect "$dcp" compatible "apple,t6041-dcpext apple,dcpext"
  expect "$dcp" status disabled
  expect "$dcp" memory-region-names "text data heap"
done
present /dcp@3a2e00000 hdmi-hpd-gpios

# Each coprocessor's firmware memory: a placeholder the boot loader fills
# in, which the kernel refuses to use until it has.
for ext in dcpext0 dcpext1; do
  for segment in text data heap; do
    expect "/reserved-memory/$ext-$segment" compatible apple,asc-mem
    expect "/reserved-memory/$ext-$segment" status disabled
  done
done

# The crossbars that route a coprocessor to each USB-C port.
for mux in /mux@38324c000 /mux@38b24c000 /mux@39324c000 /mux@39b24c000; do
  present "$mux"
done

[ -z "$failed" ]
