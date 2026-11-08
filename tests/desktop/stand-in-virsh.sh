# shellcheck shell=bash
# Stands in for virsh as the launcher's VMs run it (modules/libvirt/Machines.qml),
# since the VM runs no libvirt: the session's connection holds one machine,
# web, and the system's none. Every run is kept in virsh-asked in the
# runtime folder, a line each, so a test sees what the launcher asked and
# how. web is shut off until it's started, and virsh-web there holds the
# state it was left in. Following its events waits forever, as virsh does
# while nothing changes.
asked=$XDG_RUNTIME_DIR/virsh-asked
state=$XDG_RUNTIME_DIR/virsh-web
uuid=3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10
echo "$*" >>"$asked"

# The connection asked, and whether it was read-only, then what was asked.
writes=yes
if [ "$1" = -r ]; then
  writes=no
  shift
fi
uri=$2
shift 2

# What dominfo says of web, as libvirt 12.7 writes it with LC_ALL=C: no Id
# or processor time while it's off.
describe() {
  local now
  now=$(cat "$state" 2>/dev/null || echo "shut off")
  if [ "$now" = running ]; then
    printf '%-16s%s\n' "Id:" 1
  else
    printf '%-16s%s\n' "Id:" -
  fi
  printf '%-16s%s\n' "Name:" web "UUID:" "$uuid" "OS Type:" hvm "State:" "$now" \
    "CPU(s):" 2
  if [ "$now" = running ]; then
    printf '%-16s%s\n' "CPU time:" 12.3s
  fi
  printf '%-16s%s\n' "Max memory:" "4194304 KiB" "Used memory:" "2097152 KiB" \
    "Persistent:" yes "Autostart:" disable "Managed save:" no "Security model:" none \
    "Security DOI:" 0
  echo
}

case "$uri $*" in
"qemu:///session list --all --name") printf 'web\n\n' ;;
"qemu:///system list --all --name") printf '\n' ;;
"qemu:///session dominfo --domain web") describe ;;
*" event --loop --event lifecycle") exec sleep infinity ;;
"qemu:///session start --domain $uuid")
  [ "$writes" = yes ] && echo running >"$state"
  echo "Domain '$uuid' started"
  ;;
"qemu:///session destroy --domain $uuid" | "qemu:///session shutdown --domain $uuid")
  [ "$writes" = yes ] && echo "shut off" >"$state"
  ;;
*)
  echo "error: failed to get domain" >&2
  exit 1
  ;;
esac
