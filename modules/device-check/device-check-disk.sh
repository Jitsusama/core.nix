# Keeps what smartd heard from a disk where device-check, which each person
# at the machine runs, can read it. smartd runs it as root with the fault in
# its SMARTD_ variables, once for each kind of fault a disk reports, and reads
# anything it prints as the hook going wrong, so it prints nothing.
#
#   device-check-disk    keeps the fault smartd names, one file a kind
#
# /run empties at every boot, and smartd names each fault again once it
# starts, so what's kept is what the disks report this boot.

dir=${DEVICE_CHECK_ROOT:-}/run/device-check/disks
device=${SMARTD_DEVICE:?}
kind=${SMARTD_FAILTYPE:?}
message=${SMARTD_MESSAGE:?}

# smartd starts its message with the device, which device-check names itself.
message=${message#"Device: $device, "}
message=${message//[$'\t\n\r']/ }

mkdir -p "$dir"
file=$dir/${device##*/}-$kind
printf '%s\t%s\n' "$device" "$message" > "$file.new"
mv "$file.new" "$file"
