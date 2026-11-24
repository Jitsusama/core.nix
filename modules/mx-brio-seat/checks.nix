# The checks of mx-brio-seat, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # Until the relay gives a picture, a graphical machine hands the MX Brio to
  # the seat as any camera: no relay holds it, no rule among its udev rules
  # takes the seat's tag off it, and a switch gives back a Brio a relay held.
  the-mx-brio-is-the-seats =
    let
      inherit (graphical.config) environment systemd;
      wiring = {
        "a relay holds the MX Brio" = !(systemd.services ? mx-brio);
        "nothing gives the seat back an MX Brio a relay held" =
          (systemd.services.mx-brio-seat.script or "") == builtins.readFile ./give-back.sh;
      };
      wrong = lib.attrNames (lib.filterAttrs (_: right: !right) wiring);
    in
    assert lib.assertMsg (wrong == [ ]) (lib.concatStringsSep "; " wrong);
    pkgs.runCommandLocal "the-mx-brio-is-the-seats" { } ''
      if grep -rlE 'ID_MODEL_ID\}=="091d".*TAG-="uaccess"' \
        ${environment.etc."udev/rules.d".source}/; then
        echo "a udev rule above keeps the MX Brio from the seat"
        exit 1
      fi
      touch $out
    '';
}
