# Runs a realm's background pi host as its module renders the unit, on a
# stand-in for its node, and wedges and crashes it: a host whose loop stops
# pinging the watchdog is killed and started again, and one that keeps
# crashing stays down once its start limit is spent rather than spinning,
# while one whose command the kernel kills for memory stays up. Only
# systemd's own manager and the kernel show these, which is why this is a
# VM test; the unit's values, both hosts', are
# pi-hosts-start-with-the-session's, and that a host killed as the watchdog
# kills one resumes everything it held is the harness's own killed-host test. The stand-in, pi-hosts-recover-host.sh,
# says ready and pings as the harness's host does. The tests are
# pi-hosts-recover-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # The host runs the stand-in for node and never reads its pi, so an empty
  # folder is the runtime its proof names.
  runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}";
  proof = builtins.toFile "pi-host-proof.json" (
    builtins.toJSON {
      kind = "pi-host-runtime-proof";
      version = 1;
      inherit runtime;
    }
  );
  node = pkgs.writeShellApplication {
    name = "node";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.systemd
    ];
    text = builtins.readFile ./pi-hosts-recover-host.sh;
  };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "a-wedged-pi-starts-again";

  nodes.machine = {
    imports = [ self.nixosModules.home-manager ];
    users.users.alice = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    # The test driver makes the kernel's OOM killer panic the machine, even
    # inside one unit's limit; here that kill is what the host must outlive.
    boot.kernel.sysctl."vm.panic_on_oom" = 0;
    home-manager.users.alice = {
      imports = [
        self.homeModules.pi-packages
        ../../../modules/pi/host.nix
      ];
      home.stateVersion = "26.05";
      jitsusama.pi.hosts.test = {
        pi = "${pkgs.emptyDirectory}/pi";
        node = lib.getExe node;
        inherit proof;
      };
      # Two minutes is the host's watchdog; a wedge is the same at five
      # seconds, and the test waits on it rather than on the clock.
      systemd.user.services.pi-host-test-background.Service.WatchdogSec = lib.mkForce "5s";
    };
  };

  tests = ./pi-hosts-recover-vm-test.py;
}
