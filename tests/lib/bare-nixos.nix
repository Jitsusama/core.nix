# The least a NixOS machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as `core.lib.bareNixosModule`:
# home-manager and nixpkgs' settings, which every machine has, and one
# account, someone, for home-manager to configure, so the home half of a role
# or module is evaluated too. A check calls `nixpkgs.lib.nixosSystem` with it
# and the modules it checks, since core builds no machine itself. It is only
# for checks, never a machine's base: it allows the unfree packages core's
# nixpkgs settings and modules name, and its account has no password, so
# nobody logs in with one. Its platform, boot loader, disk and state versions give way even
# to a module's defaults, so a module checked on it may set its own.
{ self }:
{ lib, ... }:
let
  # Weaker than a module's own mkDefault, as nixos-generate-config writes
  # one, and stronger than an option's own default, such as grub's.
  bare = lib.mkOverride 1400;
in
{
  imports = [
    self.nixosModules.home-manager
    self.nixosModules.nixpkgs
  ];

  nixpkgs.hostPlatform = bare "x86_64-linux";
  boot.loader.grub.enable = bare false;
  fileSystems."/" = bare {
    device = "none";
    fsType = "tmpfs";
  };
  users.users.someone.isNormalUser = true;
  home-manager.users.someone.home.stateVersion = bare "26.05";
  system.stateVersion = bare "26.05";
}
