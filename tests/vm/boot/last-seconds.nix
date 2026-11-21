# Boots a machine that keeps its last seconds, reads this boot's ring as an
# administrator and as an account that isn't one, stops and starts the
# recorder, kills it, puts it to sleep and wakes it and sees it go on,
# powers the machine off and reads that boot as stopped after the services,
# puts it to sleep and resets it there, then
# resets it awake and reads each boot it ended. Its sleeps go as far as
# the kernel's own test of one, every process frozen and thawed, since
# QEMU's shared folders can't be put to sleep and would call a real one off.
# QEMU has no energy
# counters, battery or fans, so those records leave them out;
# modules/last-seconds/src/sample.rs's tests read them from a stand-in. The
# script is last-seconds-vm-test.py.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "last-seconds";

  nodes.machine = {
    imports = [ self.nixosModules.last-seconds ];
    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
    };
    users.users.bob.isNormalUser = true;

    # A guest under KVM has no cpuidle driver of its own, so the paravirtual
    # one, haltpoll, is made to take it, giving the recorder rest states to
    # read through its sandbox, as tests/vm/devices/deep-rest.nix does.
    boot.kernelParams = [ "cpuidle_haltpoll.force=1" ];
    boot.kernelModules = [ "cpuidle-haltpoll" ];

    # An ordinary service, ordered after basic.target as every one is,
    # that takes a while to stop and writes when it did: the recorder's
    # last record, at a power off, comes after it.
    systemd.services.stops-late = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
      };
      script = "true";
      # Two seconds, so a recorder stopped beside it rather than after it
      # would write its last record before this one's time.
      preStop = "sleep 2; date +%s%3N > /var/lib/stops-late";
    };
  };

  tests = ./last-seconds-vm-test.py;
}
