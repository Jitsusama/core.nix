# A NixOS machine Joel writes code on: the TPM for his keys and the YubiKey to
# get them back, with the workstation role in every account, and the pieces
# only Linux needs: Rust linking with mold, ssh-tpm-agent for the TPM's SSH
# keys, and commits signed with them.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.memory
    nixosModules.nix-daemon
    nixosModules.perf
    nixosModules.tpm
    nixosModules.yubikey
  ];
  home-manager.sharedModules = [
    homeModules.workstation
    homeModules.cargo
    homeModules.ssh-tpm-agent
    homeModules.signing
  ];
}
