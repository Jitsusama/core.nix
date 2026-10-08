# A Mac workstation, written the way a machine repository writes one: it
# reaches core.nix only through its outputs and builds with the stock
# darwinSystem. `nix flake check` evaluates it, so it can't drift from what the
# outputs offer.
{ core, nix-darwin }:
nix-darwin.lib.darwinSystem {
  modules = [
    core.darwinModules.workstation
    core.darwinModules.graphical
    (
      { lib, pkgs, ... }:
      {
        networking.hostName = "example";
        nixpkgs.hostPlatform = "aarch64-darwin";

        system.primaryUser = "joel";
        users.users.joel.home = "/Users/joel";
        home-manager.users.joel = {
          home.stateVersion = "26.05";

          # Setting one of core's options, the way a machine is meant to
          # change what core expects to vary.
          jitsusama.identity.email = "joel@example.com";

          # Adding to what core set: lists and attribute sets merge.
          home.packages = [ pkgs.hello ];

          # Replacing one value core set, which is rarely needed.
          programs.git.settings.pull.rebase = lib.mkForce false;

          # Removing a module a role brought in.
          disabledModules = [ core.homeModules.bat ];
        };

        system.stateVersion = 6;
      }
    )
  ];
}
