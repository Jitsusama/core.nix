# Turns a hang into a panic and a restart, so it leaves a record and the
# machine comes back by itself. A kernel that hangs with no panic writes
# nothing to pstore and loses its last half minute of journal, and the
# machine sits there until the power button is held. It's for chasing a
# machine that hangs, imported by the machine while that's chased.
#
# - A core stuck with interrupts off for ten seconds, which the kernel's
#   NMI watchdog sees, panics rather than only saying so. The panic's
#   words go to pstore, which systemd-pstore copies to
#   /var/lib/systemd/pstore at the next boot, and to netconsole when the
#   machine has it.
# - The kernel restarts 10 s after any panic, from the kernel's command
#   line, so a panic while the disk opens restarts too.
# - The chipset's watchdog, which systemd feeds while it runs, resets the
#   machine after a minute without it, for a hang too deep for a panic.
#   A minute rides out the stalls a busy machine has, of seconds.
#
# A core stuck with interrupts on, a soft lockup, still only warns: a busy
# machine has stalls of seconds, and a panic would make each a restart. So
# does a task stuck waiting on a driver while the cores still run: the
# watchdog stays fed, the machine stays hung, and only the kernel's first
# line about it is sent, without the stack that would name the driver
# (netconsole's nixos.nix says why).
{
  lib,
  pkgs,
  config,
  ...
}:
{
  # On the command line, so a panic in the initrd restarts too; and once up,
  # so a panic= named after it on the command line can't shorten it.
  boot.kernelParams = [ "panic=10" ];
  boot.kernel.sysctl = {
    "kernel.panic" = 10;
    "kernel.hardlockup_panic" = 1;
    # perf turns the NMI watchdog off to have one more counter, and with it
    # off no hard lockup is ever seen; while a hang is chased, the watchdog
    # wins.
    "kernel.nmi_watchdog" = lib.mkForce 1;
  };
  # A machine's hardware that sets its own watchdog time keeps it.
  systemd.settings.Manager.RuntimeWatchdogSec = lib.mkDefault "60s";

  # The watchdog stops for a hibernation and starts again after, and a
  # suspend is left to the driver; the script says why. systemd-sleep runs
  # it with a PATH that has no coreutils, so it brings its own.
  environment.etc."systemd/system-sleep/watchdog-sleeps".source =
    let
      hook = pkgs.writeShellApplication {
        name = "watchdog-sleeps";
        runtimeInputs = [
          config.systemd.package
          pkgs.coreutils
        ];
        text = builtins.readFile ./watchdog-sleeps.sh;
      };
    in
    "${hook}/bin/watchdog-sleeps";
}
