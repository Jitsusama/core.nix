# Where each account's SSH keys live, which the hardware decides: in the
# TPM where the machine has one (jitsusama.firmware.tpm), through
# ssh-tpm-agent, tpm-signing and link-keys; and where it has none, Joel's
# signing key on his YubiKey and the agents' key and each realm's link key
# as files on the encrypted disk, through yubikey-signing and
# file-link-keys. Both serve the same sockets, so nothing that signs or
# links needs to know which. A machine with no TPM signs in elsewhere with
# the spares the guide keeps on each YubiKey.
#
# It's a setting's value, not an import, so it can follow the hardware
# module's word without the module system needing config to know what to
# import.
{ config, ... }:
let
  tpm = config.jitsusama.firmware.tpm;
in
{
  imports = [ ../firmware-settings/nixos.nix ];

  home-manager.sharedModules =
    if tpm then
      [
        ../link-keys/home.nix
        ../ssh-tpm-agent/home.nix
        ../tpm-signing/home.nix
      ]
    else
      [
        ../file-link-keys/home.nix
        ../yubikey-signing/home.nix
      ];
}
