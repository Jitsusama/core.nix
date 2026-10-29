# shellcheck shell=bash
# Run as the machine shuts down. fwupd says a firmware update is waiting by
# pointing BootNext at its own boot entry, which applies the update at the
# next start. The new firmware measures differently into PCR 0, so the TPM
# would refuse the disk at every boot after it; PCR 0 comes out of the
# policy now, and the first boot on the new firmware locks it again
# (systemd-pcrlock-firmware-code). Any other BootNext leaves the policy as
# it is.

listed=$(efibootmgr)
next=$(sed -n 's/^BootNext: \([0-9A-F]\{4\}\)$/\1/p' <<<"$listed")
if [ -z "$next" ]; then
  exit 0
fi
# Boot0004* Linux Firmware Updater<tab>HD(...)
entry=$(grep "^Boot$next" <<<"$listed" | cut -f1 || true)
case "${entry:10}" in
  "Linux Firmware Updater" | "Linux-Firmware-Updater") ;;
  *) exit 0 ;;
esac

echo "fwupd has a firmware update waiting at the next start, so PCR 0 leaves the TPM's policy until the new firmware boots"
"$PCRLOCK" unlock-firmware-code
"$MAKE_POLICY"
