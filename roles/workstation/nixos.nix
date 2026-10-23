# A NixOS machine Joel writes code on: the TPM for his keys, a passkey in it
# for the web through linux-id, and the YubiKey to get them back, with the
# workstation role in every account, and the pieces only Linux needs: Rust
# linking with mold, ssh-tpm-agent for the TPM's SSH keys, commits signed with
# them, virtual machines through libvirt and containers through Podman. Builds
# churn its disk, so Btrfs keeps room for its metadata, Nix collects its
# garbage before the disk fills, crashes keep only a few days of cores, and a
# guard in the machine and in every account takes what regenerates before the
# disk fills. A device that stops working, or a disk that reports a fault, is
# named to whoever is signed in. The ErgoDox's devices are open to Joel, so it
# can be flashed. Each account keeps a watch on what its AI use costs, and its
# administrators, agents included, switch it without a password.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.btrfs
    nixosModules.coredump
    nixosModules.device-check
    nixosModules.disk-guard
    nixosModules.ergodox
    nixosModules.libvirt
    nixosModules.linux-id
    nixosModules.memory
    nixosModules.nix-daemon
    nixosModules.perf
    nixosModules.podman
    nixosModules.switch-system
    nixosModules.tpm
    nixosModules.yubikey
  ];
  home-manager.sharedModules = [
    homeModules.workstation
    homeModules.cargo
    homeModules.disk-guard
    homeModules.podman
    homeModules.spend-watch
    homeModules.ssh-tpm-agent
    homeModules.signing
  ];
}
