# A NixOS workstation, written the way a machine repository writes one: it
# reaches core.nix only through its outputs and builds with the stock
# nixosSystem. `nix flake check` evaluates it, so it can't drift from what the
# outputs offer.
{ core, nixpkgs }:
nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    (
      { lib, pkgs, ... }:
      {
        networking.hostName = "example";
        nixpkgs.hostPlatform = "x86_64-linux";

        # What every NixOS machine needs to boot. A real machine's come from
        # its hardware.
        boot.loader.systemd-boot.enable = true;
        fileSystems."/" = {
          device = "/dev/disk/by-label/nixos";
          fsType = "btrfs";
        };

        users.users.joel.isNormalUser = true;
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

        system.stateVersion = "26.05";
      }
    )
  ];
}
