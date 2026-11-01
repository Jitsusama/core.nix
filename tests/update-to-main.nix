# A machine updates itself to its repository's main when one of its askers
# asks, from a real NixOS switch, and nothing else moves it
# (modules/update-to-main). The repository is a bare one on the machine,
# whose flake gives one of the machine's specialisations as its system, so a
# commit chooses which system main is. machine trusts main by its signature;
# hosted trusts what its repository names, reading its objects from
# someone's checkout and asking for a token as someone, the flake in a
# folder of the repository, and builds only light pieces itself, having no
# builders; following's flake has an input, core, that follows its own
# signed main, whose file names the system. The tests are update-to-main-vm-test.py, which
# pytest runs inside the driver (tests/lib/vm_pytest.py).
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
  # Says in its home who asked for the token, and their groups, and gives
  # none while /etc/no-token is there.
  token = pkgs.writeShellScript "token" ''
    if [ -e /etc/no-token ]; then echo "no login" >&2; exit 3; fi
    echo "$(${pkgs.coreutils}/bin/id -un) $(${pkgs.coreutils}/bin/id -Gn)" > "$HOME/token-asker"
    echo a-token
  '';
  # What both machines are: whole only while not marked broken, which one
  # of the specialisations does, and building main's system in the sandbox.
  machine = extra: {
    imports = [
      self.nixosModules.update-to-main
      extra
    ];

    jitsusama.update-to-main = {
      output = "system";
      askers = [ "someone" ];
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
    # The tests' own commands need git. It's on the system's path then, so
    # Nix would find it there; a machine with none is the unit tests'.
    environment.systemPackages = [ pkgs.git ];

    # A test machine's disk was never made to boot from, so GRUB can't be
    # installed on it; a switch with no boot loader still switches.
    boot.loader.grub.enable = false;
    # The systems main may be: one that adds a file, and one that marks the
    # machine unwhole.
    specialisation.switched.configuration.environment.etc.switched.text = "switched\n";
    specialisation.unhealthy.configuration.environment.etc.unhealthy.text = "unhealthy\n";
    # One a switch by hand goes to, whose update-to-main is a copy of the
    # first's at another path, as a newer one would be.
    specialisation.moved.configuration.jitsusama.update-to-main.package =
      pkgs.runCommand "update-to-main-moved" { }
        ''
          mkdir -p $out/bin
          cp ${pkgs.callPackage ../modules/update-to-main/package.nix { }}/bin/update-to-main $out/bin/
        '';
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "update-to-main";

  nodes.machine = machine {
    jitsusama.update-to-main = {
      repository = "file:///srv/machines.git";
      signers.ssh = [ ''tester namespaces="git" ${signer}'' ];
    };
  };

  nodes.hosted = machine {
    jitsusama.update-to-main = {
      repository = "file:///srv/machines.git";
      hosted = {
        user = "someone";
        checkout = "/home/someone/machines";
        directory = "machine";
        token = [ "${token}" ];
      };
      elsewhere = true;
    };
  };

  nodes.following = machine {
    jitsusama.update-to-main = {
      repository = "file:///srv/machines.git";
      signers.ssh = [ ''tester namespaces="git" ${signer}'' ];
      follows.core = {
        repository = "file:///srv/core.git";
        signers.ssh = [ ''tester namespaces="git" ${signer}'' ];
      };
    };
  };

  tests = ./update-to-main-vm-test.py;
  inherit values;
}
