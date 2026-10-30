# Firmware updates for the Surface Laptop 4 with AMD, through fwupd. LVFS
# carries nothing for this model: Microsoft ships its firmware as UEFI
# capsules inside a package of Windows drivers. So package.nix takes Surface
# UEFI's capsule out into a folder that fwupd reads as a remote of its own,
# and fwupd offers it as it would an update from LVFS. fwupd trusts a folder
# remote as part of the system, as it does the firmware a system ships, so
# the cabinet needs no signature of its own; the capsule inside carries
# Microsoft's, which the firmware checks before it writes anything.
#
# The package's other capsules stay out. The TPM's waits on knowing whether
# its upgrade keeps the TPM's owner state, which the disk's key is sealed to;
# the Aggregator's, the touchscreen's and the others' wait on knowing which
# are newer than what the laptop runs, which its ESRT says.
{ pkgs, ... }:
let
  uefi = pkgs.callPackage ./package.nix { };
in
{
  imports = [ ../../../modules/fwupd/nixos.nix ];

  environment.etc."fwupd/remotes.d/surface-laptop-4-amd.conf".text = ''
    [fwupd Remote]
    Enabled=true
    Title=Surface Laptop 4 with AMD, from Microsoft's drivers and firmware
    MetadataURI=file://${uefi}
    ApprovalRequired=false
  '';
}
