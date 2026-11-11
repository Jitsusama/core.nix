# Lets the machine's administrators, the accounts in wheel, switch it to a
# NixOS system already built: `switch-system <system>`. A system the machine
# has run before switches with no password, and any other asks an
# administrator to authenticate. Agents switch the machine as Joel's account,
# and a prompt they can't answer stopped every switch until he came back.
#
# Being able to switch to a system one built is being root, however narrow
# the command: the system can be anything, a root shell's service included.
# So anything running as the account, a bad package among it, could have
# built itself a root shell and switched to it, unasked. A system the machine
# has run is one of the system profile's generations, which only root makes:
# through update-to-main, which brings new systems from main, or through a
# switch an administrator authenticated. Going back to one of those, or on to
# what update-to-main brought, gives the account nothing it chose itself, so
# it stays unasked; a system the account built asks once, and is then one the
# machine has run. It refuses what isn't a NixOS system in the store, which
# guards against mistakes, not against the account.
#
# The switch runs as an instance of switch-system@.service, named by the
# system's path, as nixos-rebuild runs its own in a unit of its own: a switch
# restarts what changed, and run from the account's session it could stop
# what's running it. polkit lets wheel start that unit and nothing else.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # What the unit runs, as root (switch-as-root.sh).
  switch = pkgs.writeShellApplication {
    name = "switch-system-as-root";
    runtimeInputs = [ config.nix.package ];
    text = builtins.readFile ./switch-as-root.sh;
  };

  # What polkit asks whether a switch is to a system the machine has run
  # (switch-system-known.sh).
  known = pkgs.writeShellApplication {
    name = "switch-system-known";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.gnugrep
      config.systemd.package
    ];
    text = builtins.readFile ./switch-system-known.sh;
  };

  # What the account runs (switch-system.sh).
  ask = pkgs.writeShellApplication {
    name = "switch-system";
    runtimeInputs = [
      pkgs.coreutils
      config.systemd.package
    ];
    text = builtins.readFile ./switch-system.sh;
  };
in
{
  systemd.services."switch-system@" = {
    description = "Switch to the NixOS system at %f";
    # A switch restarts what changed, and this is what's switching.
    restartIfChanged = false;
    stopIfChanged = false;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${switch}/bin/switch-system-as-root %f";
    };
  };

  # Without polkit, systemd lets only root start a unit.
  security.polkit.enable = true;
  security.polkit.extraConfig = builtins.replaceStrings [ "@known@" ] [ (lib.getExe known) ] (
    builtins.readFile ./switch-system.rules
  );

  environment.systemPackages = [ ask ];
}
