# shellcheck shell=bash disable=SC2154 # The shell, the refusal and QEMU are the build's attributes in nixos.nix.
# Replaces each of QEMU's emulators, in a joined copy of QEMU, with a script
# that runs the real one through the refusal of io_uring (nixos.nix says
# why). Each written script names its own shell, since a wrapper libvirt
# starts finds nothing on its path.

for emulator in "$out"/bin/qemu-system-* "$out/bin/qemu-kvm"; do
  [ -e "$emulator" ] || continue
  real=$(readlink -f "$emulator")
  rm "$emulator"
  # shellcheck disable=SC2016 # The script written says "$@" itself.
  printf '#!%s\nexec %s %s "$@"\n' "$shell" "$refuse" "$real" > "$emulator"
  chmod +x "$emulator"
done

# libvirt's module finds the firmware by this package's own path.
for descriptor in "$out"/share/qemu/firmware/*.json; do
  real=$(readlink -f "$descriptor")
  rm "$descriptor"
  substitute "$real" "$descriptor" --replace-quiet "$qemu" "$out"
done
