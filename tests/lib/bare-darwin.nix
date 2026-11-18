# The least a nix-darwin machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as
# `core.lib.bareDarwinModule`: home-manager and nixpkgs' settings, which every
# machine has, and one account, someone, for home-manager to configure. A
# check calls `nix-darwin.lib.darwinSystem` with it and the modules it
# checks, since core builds no machine itself. It is only for checks, never a
# machine's base: it allows unfree packages, as core's nixpkgs settings do.
# Its platform and primary user are defaults, so a module checked on it may
# set its own.
{ self }:
{ lib, ... }:
{
  imports = [
    self.darwinModules.home-manager
    self.darwinModules.nixpkgs
  ];

  nixpkgs.hostPlatform = lib.mkDefault "aarch64-darwin";
  system.primaryUser = lib.mkDefault "someone";
  users.users.someone.home = "/Users/someone";
  home-manager.users.someone.home.stateVersion = "26.05";
  system.stateVersion = 6;
}
