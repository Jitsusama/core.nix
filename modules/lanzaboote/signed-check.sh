# shellcheck shell=bash
# Fails unless every program the firmware starts from the ESP ($ESP_PATH)
# is signed by the machine's keys, and then takes the keys' files
# ($AUTO_KEYS) away, so systemd-boot doesn't enrol them at the next start
# (nixos.nix says which programs, and why).
started='[.[] | select(.file_name | test("/EFI/BOOT/BOOTX64\\.EFI$|/EFI/systemd/systemd-bootx64\\.efi$|/EFI/Linux/[^/]+\\.efi$"; "i"))]'
# sbctl says why it failed on its standard output.
if ! verified=$(sbctl verify --json); then
  echo "sbctl couldn't check the ESP, so the keys aren't enrolled: $verified" >&2
  rm -r "$AUTO_KEYS"
  exit 1
fi
if ! jq -e "$started | length > 0 and all(.[]; .is_signed == 1)" <<< "$verified" > /dev/null; then
  echo "A program the firmware starts isn't signed, so the keys aren't enrolled:" >&2
  jq -r "$started | .[] | select(.is_signed != 1) | .file_name" <<< "$verified" >&2
  rm -r "$AUTO_KEYS"
  exit 1
fi
