# Lets an account in wheel trace its own niri's main thread off the CPU,
# with no password: `niri-off-cpu <seconds>`. When niri misses frames under
# load without waiting for a CPU, what it waited on is a kernel stack, and
# niri's file capability keeps even its own account from reading that, so
# only root can trace it, and agents can't type a password.
#
# The trace runs only while asked, as an instance of niri-off-cpu@.service
# named by the account, which polkit lets that account start and stop and no
# other. Reading the kernel's stacks as root is something a password in wheel
# already gives, so this grants nothing new; the script it runs is fixed and
# in the store, and it traces nothing but the account's own compositor.
{ config, pkgs, ... }:
let
  # What the unit runs, as root (off-cpu-as-root.sh).
  trace = pkgs.writeShellApplication {
    name = "niri-off-cpu-as-root";
    runtimeInputs = [
      pkgs.bpftrace
      pkgs.coreutils
      pkgs.procps
    ];
    text = builtins.readFile ./off-cpu-as-root.sh;
  };

  # What the account runs (niri-off-cpu.sh).
  ask = pkgs.writeShellApplication {
    name = "niri-off-cpu";
    runtimeInputs = [
      pkgs.coreutils
      config.systemd.package
    ];
    text = builtins.readFile ./niri-off-cpu.sh;
  };
in
{
  systemd.services."niri-off-cpu@" = {
    description = "Trace %i's niri off the CPU";
    environment.OFF_CPU = "${./off-cpu.bt}";
    serviceConfig = {
      Type = "exec";
      ExecStart = "${trace}/bin/niri-off-cpu-as-root %i";
      # bpftrace writes what it holds and runs END on an interrupt.
      KillSignal = "SIGINT";
      # A trace left running by an asker that went away ends anyway.
      RuntimeMaxSec = "10min";
      RuntimeDirectory = "niri-off-cpu";
      RuntimeDirectoryPreserve = true;
    };
  };

  # Without polkit, systemd lets only root start a unit.
  security.polkit.enable = true;
  security.polkit.extraConfig = builtins.readFile ./niri-off-cpu.rules;

  environment.systemPackages = [ ask ];
}
