# Runs a realm's attended pi host as its module renders the unit, on the
# harness's own host program and a pi built with the host's patches, holding
# two conversations, one parked and one idle, and takes the host away twice:
# a kill -9 of its process, and a reboot of the machine. Each time the
# account's own systemd brings the host back, which resumes both from their
# records, and after the reboot a viewer attaching through mycelium is drawn
# each one's screen. Only the account's manager and a real boot show that,
# which is why this is a VM test; the harness's own killed-host and
# release cases prove the same resumption in a host started by hand. The
# conversations answer the harness's scripted model, so nothing reaches a
# network. The tests are pi-conversations-come-back-vm-test.py, which pytest
# runs inside the driver.
{
  self,
  pkgs,
  # A pi built with the host's patches (modules/pi/hosted-pi.nix), and its
  # package folder, where its dist and node_modules are.
  hostedPi,
  packageDir,
}:
let
  inherit (pkgs) lib;
  # The harness commit these cases were written against. Its host program
  # needs nothing beyond pi's own packages but zod, and its park workflow and
  # scripted model nothing at all, so it runs on pi's node_modules, linked in,
  # rather than on a build of its whole tree, whose native modules a host
  # never loads.
  commit = "c365e2c19e515074dad1317c8356008cddaaf7b6";
  source = pkgs.fetchFromGitHub {
    owner = "Jitsusama";
    repo = "agentic-harness.pi";
    rev = commit;
    hash = "sha256-opAUDZPkNqbXAYhkuXduvzwmDRwAhh0fpIl9Vp1MGPI=";
  };
  harness = pkgs.runCommand "agentic-harness-${lib.substring 0 12 commit}" { } ''
    cp -r ${source} $out
    chmod u+w $out
    ln -s ${packageDir}/node_modules $out/node_modules
  '';
  # The proof the host checks as it starts, naming the runtime its pi is in;
  # its counts are the harness's record of a run, never read back.
  proof = builtins.toFile "pi-host-proof.json" (
    builtins.toJSON {
      kind = "pi-host-runtime-proof";
      version = 1;
      runtime = builtins.unsafeDiscardStringContext "${hostedPi}";
      harness = commit;
      files = 36;
      cases = 152;
      provenAt = "2026-11-12T09:00:00.000Z";
    }
  );
  # The node pi's own build ran on, which buildNpmPackage takes as nodejs.
  node = lib.getExe pkgs.nodejs;
  ask = ./pi-conversations-come-back-ask.mjs;
  # The viewer that attaches to a hosted conversation.
  viewer = self.inputs.mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "pi-conversations-come-back";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    # A pi host holds every conversation in one node, with its screen.
    virtualisation.memorySize = 2048;
    # The account lingers, as the one this was written for does, so its
    # manager and the hosts it starts come up with the machine.
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    # The tests install the harness and ask the host from the machine itself.
    system.extraDependencies = [
      harness
      pkgs.nodejs
      ask
      viewer
    ];
    environment.systemPackages = [ pkgs.util-linux ];
    home-manager.users.alice = {
      imports = [
        self.homeModules.pi-packages
        ../../../modules/pi/host.nix
      ];
      home.stateVersion = "26.05";
      jitsusama.pi.hosts.work = {
        pi = packageDir;
        inherit node proof;
        environment = {
          # The scripted model takes pi-ai's faux provider from here.
          PI_HOST_PACKAGE_DIR = packageDir;
          PI_HOME_REALM = "work";
          PI_OFFLINE = "1";
          PI_SKIP_VERSION_CHECK = "1";
        };
      };
    };
  };

  values = pkgs.writeText "pi-conversations-come-back-values.json" (
    builtins.toJSON {
      inherit
        commit
        harness
        ask
        node
        ;
      mycelium = lib.getExe viewer;
    }
  );

  tests = ./pi-conversations-come-back-vm-test.py;
}
