# A machine updates itself to its repository's main when one of its askers
# asks, from a real NixOS switch, and nothing else moves it
# (modules/update-to-main). The repository is a bare one on the machine,
# whose flake gives one of the machine's specialisations as its system, so a
# commit chooses which system main is. The tests are
# update-to-main-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # The key the tests sign main with, made for them and trusted nowhere
  # else (update-to-main-signer).
  signer = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAID4JfIRFzk0AAx0rVEpD5mF9Vt+mlZd6CtKZowiuoF4C";
  values = pkgs.writeText "update-to-main-values.json" (
    builtins.toJSON {
      key = ./update-to-main-signer;
    }
  );
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "update-to-main";

  nodes.machine = {
    imports = [ self.nixosModules.update-to-main ];

    jitsusama.update-to-main = {
      repository = "file:///srv/machines.git";
      output = "system";
      signers.ssh = [ ''tester namespaces="git" ${signer}'' ];
      askers = [ "someone" ];
      # A machine is whole only while it isn't marked broken, which one of
      # the specialisations does.
      health = [
        [
          "${pkgs.coreutils}/bin/test"
          "!"
          "-e"
          "/etc/unhealthy"
        ]
      ];
      healthWithin = 5;
    };

    nix.settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      # Nothing is fetched: the systems are on the machine already.
      substituters = pkgs.lib.mkForce [ ];
      # main's system is a link made in the sandbox, as a real system is
      # built, by a busybox needing nothing else there.
      extra-sandbox-paths = [ "/bin/busybox=${pkgs.pkgsStatic.busybox}/bin/busybox" ];
    };

    users.users.someone.isNormalUser = true;
    users.users.other.isNormalUser = true;
    environment.systemPackages = [ pkgs.git ];

    # A test machine's disk was never made to boot from, so GRUB can't be
    # installed on it; a switch with no boot loader still switches.
    boot.loader.grub.enable = false;
    # The systems main may be: one that adds a file, and one that marks the
    # machine unwhole.
    specialisation.switched.configuration.environment.etc.switched.text = "switched\n";
    specialisation.unhealthy.configuration.environment.etc.unhealthy.text = "unhealthy\n";
  };

  tests = ./update-to-main-vm-test.py;
  inherit values;
}
