# The least a NixOS machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as `core.lib.bareNixosModule`:
# home-manager and nixpkgs' settings, which every machine has, and one
# account, someone, for home-manager to configure, so the home half of a role
# or module is evaluated too. A check calls `nixpkgs.lib.nixosSystem` with it
# and the modules it checks, since core builds no machine itself.
{ self }:
{ lib, ... }:
{
  imports = [
    self.nixosModules.home-manager
    self.nixosModules.nixpkgs
  ];

  nixpkgs.hostPlatform = "x86_64-linux";
  boot.loader.grub.enable = false;
  # A module that lays out the disk replaces this.
  fileSystems."/" = lib.mkDefault {
    device = "none";
    fsType = "tmpfs";
  };
  users.users.someone.isNormalUser = true;
  home-manager.users.someone.home.stateVersion = "26.05";
  system.stateVersion = "26.05";
}
