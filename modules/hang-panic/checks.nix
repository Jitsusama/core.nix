# The checks of hang-panic, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareNixos,
  onLinux,
  pkgs,
  self,
  ...
}:
onLinux {
  # The sleep hook, as systemd-sleep runs it with an empty PATH, against a
  # stand-in of busctl: it stops a running watchdog for each step that
  # hibernates and hands it back to systemd's setting after, leaves a
  # suspend to the driver, suspend-then-hibernate's first step included, and
  # starts nothing that wasn't running. The VM test proves it on systemd's
  # own watchdog; this proves the cases it doesn't reach.
  hang-panic-sleeps-whole =
    let
      machine = bareNixos [ self.nixosModules.hang-panic ];
      hook = machine.config.environment.etc."systemd/system-sleep/watchdog-sleeps".source;
    in
    pkgs.runCommandLocal "hang-panic-sleeps-whole" { } ''
      ${pkgs.bash}/bin/bash ${./sleeps-whole.sh} ${hook}
      touch $out
    '';
}
