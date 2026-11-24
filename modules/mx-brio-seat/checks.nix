# The checks of mx-brio-seat, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  graphical,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # A graphical machine that shares the Brio, whose rules the search below
  # must find, so it's shown to catch the rule it's there for.
  shared = graphical.extendModules { modules = [ self.nixosModules.mx-brio ]; };
  run = pkgs.runCommandLocal;
in
onLinux {
  # Until the relay gives a picture, a graphical machine hands the MX Brio to
  # the seat as any camera: no relay holds it, no rule among its udev rules
  # marks it relayed or takes the seat's tag off it, as the sharing's rule
  # does (tests/no-rule-hides-the-brio.sh), and every boot, and a switch
  # bringing it in, starts the give-back, which gives back a Brio a relay
  # held. udev's tool is on every unit's path, as NixOS's default path holds
  # systemd, so nothing here asks for it.
  the-mx-brio-is-the-seats =
    let
      inherit (graphical.config) environment systemd;
      give = systemd.services.mx-brio-seat or { };
      wiring = {
        "a relay holds the MX Brio" = !(systemd.services ? mx-brio);
        "nothing gives the seat back an MX Brio a relay held" =
          (give.script or "") == builtins.readFile ./give-back.sh;
        "nothing starts the give-back" = lib.elem "multi-user.target" (give.wantedBy or [ ]);
      };
      wrong = lib.attrNames (lib.filterAttrs (_: right: !right) wiring);
      rules = machine: machine.config.environment.etc."udev/rules.d".source;
      search = ./tests/no-rule-hides-the-brio.sh;
    in
    assert lib.assertMsg (wrong == [ ]) (lib.concatStringsSep "; " wrong);
    run "the-mx-brio-is-the-seats" { } ''
      bash ${search} ${rules shared} ${environment.etc."udev/rules.d".source}
      touch $out
    '';

  # The give-back replays the Brio alone, never another camera a relay
  # holds, bounds each wait on udev within the unit's own limit, ends well
  # whether udev answers, refuses or never settles, and says it gave the
  # Brio back only when udev's record shows it did, run under set -e as
  # its unit runs it, against a stand-in udevadm
  # (tests/give-back-never-fails.sh).
  the-mx-brio-give-back-never-fails =
    let
      limit = graphical.config.systemd.services.mx-brio-seat.serviceConfig.TimeoutStartSec;
      # The unit's limit in seconds, which the stand-in holds each wait to,
      # read only from whole minutes: systemd takes a bare number as seconds,
      # which read as minutes would let a wait sixty times too long pass.
      minutes = builtins.match "([0-9]+)min" limit;
      seconds = lib.toInt (builtins.head minutes) * 60;
    in
    assert lib.assertMsg (minutes != null) "mx-brio-seat's TimeoutStartSec isn't whole minutes";
    run "the-mx-brio-give-back-never-fails" { } ''
      bash ${./tests}/give-back-never-fails.sh ${./give-back.sh} ${toString seconds}
      touch $out
    '';
}
