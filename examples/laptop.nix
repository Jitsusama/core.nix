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
    core.nixosModules.disk-loss-panic
    core.nixosModules.disko
    core.nixosModules.lanzaboote
    # The modules no role brings, which a machine names itself, so every
    # module core offers meets the others here before it meets a machine:
    # two that clash, such as two layers claiming one chord, fail this.
    core.nixosModules.chrome
    core.nixosModules.guide
    core.nixosModules.inlyne
    core.nixosModules.update-to-main
    (
      { config, ... }:
      {
        # While a disk that goes is chased, it panics the kernel once the
        # disk stops answering, so pstore keeps the kernel's log.
        jitsusama.diskLossPanic.disk = config.jitsusama.disk.device;
      }
    )
    {
      networking.hostName = "example-laptop";
      nixpkgs.hostPlatform = "x86_64-linux";

      # The disk is a fact about this one machine, not its model.
      jitsusama.disk = {
        device = "/dev/disk/by-id/nvme-example";
        swapSize = "64G";
      };

      jitsusama.update-to-main = {
        repository = "https://github.com/example/machines";
        output = "nixosConfigurations.example-laptop.config.system.build.toplevel";
      };

      users.users.joel.isNormalUser = true;
      home-manager.users.joel = {
        home.stateVersion = "26.05";
        imports = [
          core.homeModules.cadence
          core.homeModules.chrome
          core.homeModules.merge-net
          core.homeModules.network-revive
          core.homeModules.passwords
          core.homeModules.people
          core.homeModules.session-backup
          core.homeModules.slack
          core.homeModules.wifid
          core.homeModules.zoom
        ];
      };

      system.stateVersion = "26.05";
    }
  ];
}
