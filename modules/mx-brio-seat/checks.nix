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
in
onLinux {
  # Until the relay gives a picture, a graphical machine hands the MX Brio to
  # the seat as any camera: no relay holds it, no rule among its udev rules
  # marks it relayed or takes the seat's tag off it, as the sharing's rule
  # does, and every boot and switch starts the give-back with udev's tool to
  # hand, which gives back a Brio a relay held.
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
    in
    assert lib.assertMsg (wrong == [ ]) (lib.concatStringsSep "; " wrong);
    pkgs.runCommandLocal "the-mx-brio-is-the-seats" { } ''
      # A rule marking a camera relayed, or taking the Brio's seat tag off,
      # in either order; -R follows the rules' links into the store.
      hides() {
        grep -RlE -e 'CAMERA_RELAYED\}="1"' \
          -e '091d.*TAG-="uaccess"' -e 'TAG-="uaccess".*091d' "$1"/
      }
      if ! hides ${rules shared} >/dev/null; then
        echo "the search finds no rule hiding the MX Brio even where the sharing's is"
        exit 1
      fi
      if hides ${environment.etc."udev/rules.d".source}; then
        echo "a udev rule above keeps the MX Brio from the seat"
        exit 1
      fi
      touch $out
    '';
}
