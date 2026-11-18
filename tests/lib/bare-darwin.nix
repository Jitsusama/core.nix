# The least a nix-darwin machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as
# `core.lib.bareDarwinModule`: home-manager and nixpkgs' settings, which every
# machine has, and one account, someone, for home-manager to configure. A
# check calls `nix-darwin.lib.darwinSystem` with it and the modules it
# checks, since core builds no machine itself. It is only for checks, never a
# machine's base: it allows unfree packages, as core's nixpkgs settings do.
# Its platform, primary user and state versions give way even to a module's
# defaults, so a module checked on it may set its own.
{ self }:
{ lib, ... }:
let
  # Weaker than a module's own mkDefault, and stronger than an option's own
  # default.
  bare = lib.mkOverride 1400;
in
{
  imports = [
    self.darwinModules.home-manager
    self.darwinModules.nixpkgs
  ];

  nixpkgs.hostPlatform = bare "aarch64-darwin";
  system.primaryUser = bare "someone";
  users.users.someone.home = "/Users/someone";
  home-manager.users.someone.home.stateVersion = bare "26.05";
  system.stateVersion = bare 6;
}
