# A NixOS laptop on hardware core.nix knows, with the encrypted disk and
# Secure Boot every machine Joel carries uses. `nix flake check` evaluates
# it, so a machine like it can't drift from what the outputs offer.
{ core, nixpkgs }:
nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    core.nixosModules.laptop
    core.nixosModules.dell-xps-14-da14260
    core.nixosModules.disko
    core.nixosModules.lanzaboote
    {
      networking.hostName = "example-laptop";
      nixpkgs.hostPlatform = "x86_64-linux";

      # The disk is a fact about this one machine, not its model.
      jitsusama.disk = {
        device = "/dev/disk/by-id/nvme-example";
        swapSize = "64G";
      };

      users.users.joel.isNormalUser = true;
      home-manager.users.joel.home.stateVersion = "26.05";

      system.stateVersion = "26.05";
    }
  ];
}
