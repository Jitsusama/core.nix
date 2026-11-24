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
  # bringing it in, starts the give-back with udev's tool to hand, which
  # gives back a Brio a relay held.
  the-mx-brio-is-the-seats =
    let
      inherit (graphical.config) environment systemd;
      give = systemd.services.mx-brio-seat or { };
      wiring = {
        "a relay holds the MX Brio" = !(systemd.services ? mx-brio);
        "nothing gives the seat back an MX Brio a relay held" =
          (give.script or "") == builtins.readFile ./give-back.sh;
        "nothing starts the give-back" = lib.elem "multi-user.target" (give.wantedBy or [ ]);
        "the give-back has no udevadm" = lib.elem systemd.package (give.path or [ ]);
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
  # holds, ends well whether udev answers, refuses or never settles, and
  # says it gave the Brio back only when udev's record shows it did, run
  # against a stand-in udevadm (tests/give-back-never-fails.sh).
  the-mx-brio-give-back-never-fails = run "the-mx-brio-give-back-never-fails" { } ''
    bash ${./tests}/give-back-never-fails.sh ${./give-back.sh}
    touch $out
  '';
}
