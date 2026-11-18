# The least a nix-darwin machine needs to evaluate, as one module, for core's
# checks and for any repository built on core, as
# `core.lib.bareDarwinModule`: home-manager and nixpkgs' settings, which every
# machine has, and one account, someone, for home-manager to configure. A
# check calls `nix-darwin.lib.darwinSystem` with it and the modules it
# checks, since core builds no machine itself.
{ self }:
{
  imports = [
    self.darwinModules.home-manager
    self.darwinModules.nixpkgs
  ];

  nixpkgs.hostPlatform = "aarch64-darwin";
  system.primaryUser = "someone";
  users.users.someone.home = "/Users/someone";
  home-manager.users.someone.home.stateVersion = "26.05";
  system.stateVersion = 6;
}
