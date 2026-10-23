# Lets the machine's administrators, the accounts in wheel, switch it to a
# NixOS system already built, with no password: `switch-system <system>`.
# Agents switch the machine as Joel's account, and a prompt they can't answer
# stopped every switch until he came back.
#
# Being able to switch to a system one built is being root, however narrow
# the command: the system can be anything, a root shell's service included.
# So this grants nothing to anyone who couldn't already become root with a
# password; it only stops asking for it. It refuses what isn't a NixOS system
# in the store, which guards against mistakes, not against the account.
#
# The switch runs as an instance of switch-system@.service, named by the
# system's path, as nixos-rebuild runs its own in a unit of its own: a switch
# restarts what changed, and run from the account's session it could stop
# what's running it. polkit lets wheel start that unit and nothing else.
{ config, pkgs, ... }:
let
  # What the unit runs, as root (switch-as-root.sh).
  switch = pkgs.writeShellApplication {
    name = "switch-system-as-root";
    runtimeInputs = [ config.nix.package ];
    text = builtins.readFile ./switch-as-root.sh;
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

  security.polkit.extraConfig = builtins.readFile ./switch-system.rules;

  environment.systemPackages = [ ask ];
}
