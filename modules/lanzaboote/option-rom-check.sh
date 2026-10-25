# shellcheck shell=bash
# Asks sbctl whether enrolling the machine's keys would stop the firmware
# running an option ROM it ran this boot (nixos.nix says why lanzaboote
# skips that question), and fails, saying what to set, if it would. It
# exports to a scratch directory, which needs no setup mode, so it changes
# nothing.
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
if ! output=$(cd "$scratch" && sbctl enroll-keys --export auth 2>&1); then
  printf '%s\n' "$output" >&2
  case "$output" in
    *"Found OptionROM"*)
      echo "The firmware ran an option ROM, so no keys were enrolled. Set" \
        "boot.lanzaboote.autoEnrollKeys.includeChecksumsFromTPM to keep running it," \
        "or includeMicrosoftKeys for a graphics card." >&2
      ;;
  esac
  exit 1
fi
