# The least a NixOS machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as `core.lib.bareNixosModule`:
# home-manager and nixpkgs' settings, which every machine has, and one
# account, someone, for home-manager to configure, so the home half of a role
# or module is evaluated too. A check calls `nixpkgs.lib.nixosSystem` with it
# and the modules it checks, since core builds no machine itself. It is only
# for checks, never a machine's base: it allows unfree packages, as core's
# nixpkgs settings do, and its account has no password. Its platform, boot
# loader and disk are defaults, so a module checked on it may set its own.
{ self }:
{ lib, ... }:
{
  imports = [
    self.nixosModules.home-manager
    self.nixosModules.nixpkgs
  ];

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  boot.loader.grub.enable = lib.mkDefault false;
  fileSystems."/" = lib.mkDefault {
    device = "none";
    fsType = "tmpfs";
  };
  users.users.someone.isNormalUser = true;
  home-manager.users.someone.home.stateVersion = "26.05";
  system.stateVersion = "26.05";
}
