# The checks of mx-brio, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. No VM has an MX
# Brio to plug in, nor any camera uvcvideo drives, so what's checked is the
# wiring as each piece reads it.
{
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The MX Brio is shared through its loopback on every graphical machine:
  # among the machine's own udev rules, udev reads the Brio's without a
  # complaint, and it comes after every rule tagging cameras for the seat
  # and before the handing, keeps the Brio from the seat, marks it relayed
  # and starts the relay on its capture node
  # (tests/the-brio-is-kept-from-the-seat.sh); the relay is bound to that
  # node, takes its loopback under the Brio's own name and reads the
  # Brio's NV12 at 1080p and 30 frames a second, its MJPEG only where NV12
  # can't be had; the loopback's holders file is named as the Brio is and
  # removed when the Brio goes; and the Brio's own watch writes in the
  # hidden folder, never beside the loopback's file.
  the-mx-brio-is-shared =
    let
      inherit (graphical.config) environment services systemd;
      relay = systemd.services.mx-brio or (throw "no service relays the MX Brio");
      holders = systemd.services.mx-brio-holders or (throw "no service says who holds the MX Brio");
      watch =
        systemd.services."usb-camera-holders-relayed@" or (throw "no watch records a relayed camera");
      card = "MX Brio 705 for Business";
      wiring = {
        "the relay isn't bound to the Brio's capture node" =
          lib.elem "dev-mx\\x2dbrio.device" relay.bindsTo;
        "the relay doesn't take its loopback under the Brio's name" =
          builtins.match ".*/bin/camera-loopback '${card}' /run/mx-brio/device -b 4 -x 1" relay.preStart
          != null;
        "the relay doesn't read the Brio's NV12 at 1080p and 30 frames a second first" =
          lib.hasInfix "v4l2src device=/dev/mx-brio ! video/x-raw,format=NV12,width=1920,height=1080,framerate=30/1;image/jpeg," relay.script;
        "the loopback's holders file isn't named as the Brio is" =
          lib.hasSuffix "'/run/camera-holders/${card}'" holders.serviceConfig.ExecStart;
        "the loopback's holders file stays once the Brio has gone" =
          lib.hasInfix "/bin/rm -f '/run/camera-holders/${card}'" holders.serviceConfig.ExecStopPost;
        "the Brio's own watch doesn't write in the hidden folder" =
          lib.hasSuffix " /run/camera-holders/.relayed" watch.serviceConfig.ExecStart;
        "a relayed camera's watch could write beside its loopback's" =
          lib.hasInfix ''ENV{CAMERA_RELAYED}!="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="usb-camera-holders@%k.service"'' services.udev.extraRules;
      };
      wrong = lib.attrNames (lib.filterAttrs (_: right: !right) wiring);
      udev.nativeBuildInputs = [ pkgs.systemd ];
    in
    assert lib.assertMsg (wrong == [ ]) (lib.concatStringsSep "; " wrong);
    pkgs.runCommandLocal "the-mx-brio-is-shared" udev ''
      bash ${./tests/the-brio-is-kept-from-the-seat.sh} ${environment.etc."udev/rules.d".source}
      touch $out
    '';
}
