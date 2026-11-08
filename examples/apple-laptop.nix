# A NixOS laptop on Apple silicon, the MacBook Air M2, with the roles every
# laptop Joel carries takes. `nix flake check` evaluates it, so what the
# roles hold keeps evaluating for aarch64-linux and for a machine with no
# TPM and no Secure Boot, which the x86-64 examples can't show.
{ core, nixpkgs }:
nixpkgs.lib.nixosSystem {
  modules = [
    core.nixosModules.workstation
    core.nixosModules.graphical
    core.nixosModules.laptop
    core.nixosModules.apple-macbook-air-m2-a2681
    core.nixosModules.disko
    {
      networking.hostName = "example-apple-laptop";

      # Asahi's installer leaves the EFI system partition beside macOS's
      # stub, and the system goes in the space it left free.
      jitsusama.disk = {
        device = "/dev/disk/by-id/nvme-example";
        swapSize = "24G";
        shared = true;
      };

      users.users.joel.isNormalUser = true;
      home-manager.users.joel.home.stateVersion = "26.05";

      system.stateVersion = "26.05";
    }
  ];
}
