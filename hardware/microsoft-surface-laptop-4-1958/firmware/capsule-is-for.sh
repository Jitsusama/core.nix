# shellcheck shell=bash
# Fails unless a capsule updates the firmware its GUID names, to the version
# given, as the firmware itself will read them: the image's type in its FMP
# capsule header, and the version in the payload's header Microsoft puts
# after the image's signature. So a package naming the wrong file, or a
# newer package's file under an old version, never builds.
# usage: capsule-is-for.sh <capsule> <GUID> <version, as one number>
set -euo pipefail

capsule=$1
guid=$2
version=$3

fail() {
  echo "$capsule: $*" >&2
  exit 1
}

# A little-endian number of $2 bytes at offset $1.
number() {
  od -An -tu"$2" -j"$1" -N"$2" "$capsule" | tr -d ' '
}

# $2 bytes at offset $1, as hex.
bytes() {
  od -An -tx1 -j"$1" -N"$2" "$capsule" | tr -d ' \n'
}

# A GUID's bytes as UEFI lays them out: the first three fields little-endian.
guid_bytes() {
  local hex=${1//-/}
  hex=${hex,,}
  echo "${hex:6:2}${hex:4:2}${hex:2:2}${hex:0:2}${hex:10:2}${hex:8:2}${hex:14:2}${hex:12:2}${hex:16:16}"
}

# EFI_FIRMWARE_MANAGEMENT_CAPSULE_ID_GUID, which every FMP capsule opens with.
[ "$(bytes 0 16)" = "$(guid_bytes 6dcbd5ed-e82d-4c44-bda1-7194199ad92a)" ] ||
  fail "isn't an FMP capsule"

# The capsule's header, then the FMP header with no drivers and one image,
# whose offset from the FMP header is the list's only entry.
header=$(number 16 4)
[ "$(number $((header + 4)) 2)" = 0 ] || fail "carries drivers, which this doesn't read"
[ "$(number $((header + 6)) 2)" = 1 ] || fail "carries more than one image"
image=$((header + $(number $((header + 8)) 8)))

[ "$(bytes $((image + 4)) 16)" = "$(guid_bytes "$guid")" ] ||
  fail "updates another firmware than $guid"

# The image's header is 32 bytes at version 1; version 2 adds the hardware
# instance and 3 the capsule support, 8 bytes each. The payload opens with
# the image's signature: a monotonic count, then a WIN_CERTIFICATE whose
# first field is its whole length.
case $(number "$image" 4) in
  1) payload=$((image + 32)) ;;
  2) payload=$((image + 40)) ;;
  3) payload=$((image + 48)) ;;
  *) fail "has an image header of a version this doesn't read" ;;
esac
signed=$((payload + 8 + $(number $((payload + 8)) 4)))

# FMP_PAYLOAD_HEADER: "MSS1", its own size, then the version.
[ "$(bytes "$signed" 4)" = "4d535331" ] || fail "has no payload header after its signature"
found=$(number $((signed + 8)) 4)
[ "$found" = "$version" ] || fail "is version $found, not $version"
