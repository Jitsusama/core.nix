# The checks of mx-brio, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. No VM has an MX
# Brio to plug in, nor any camera uvcvideo drives, so what's checked is the
# wiring as each piece reads it.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # A graphical machine that imports the sharing, since no role does while
  # its relay gives no picture (nixos.nix says why).
  shared = graphical.extendModules { modules = [ self.nixosModules.mx-brio ]; };
in
onLinux {
  # Until the relay gives a picture, a graphical machine hands the MX Brio
  # to the seat as any camera: no rule among its udev rules takes the seat's
  # tag off the Brio, and no relay holds it, so Chrome lists the Brio itself.
  the-mx-brio-is-the-seats =
    let
      inherit (graphical.config) environment systemd;
    in
    assert lib.assertMsg (!(systemd.services ? mx-brio)) "a relay holds the MX Brio";
    pkgs.runCommandLocal "the-mx-brio-is-the-seats" { } ''
      if grep -rlE 'ID_MODEL_ID\}=="091d".*TAG-="uaccess"' \
        ${environment.etc."udev/rules.d".source}/; then
        echo "a udev rule above keeps the MX Brio from the seat"
        exit 1
      fi
      touch $out
    '';

  # The MX Brio is shared through its loopback on a graphical machine that
  # imports it:
  # among the machine's own udev rules, udev reads the Brio's without a
  # complaint, and it comes after every rule tagging cameras for the seat
  # and before the handing, keeps the Brio from the seat, marks it relayed
  # and starts the relay on its capture node
  # (tests/the-brio-is-kept-from-the-seat.sh); the relay is bound to that
  # node, takes its loopback under the Brio's own name, and starts with
  # the plugins it's given on the Brio's NV12 at 1080p and 30 frames a
  # second and on its MJPEG, which it takes only where NV12 can't be had
  # (tests/the-relay-takes-either-format.sh); the loopback's holders file
  # is named as the Brio is and removed when the Brio goes, and the shell
  # lists the Brio only while that file is there, by its card; and the
  # Brio's own watch writes in the hidden folder, never beside the
  # loopback's file.
  the-mx-brio-is-shared =
    let
      inherit (shared.config) environment services systemd;
      relay = systemd.services.mx-brio or (throw "no service relays the MX Brio");
      holders = systemd.services.mx-brio-holders or (throw "no service says who holds the MX Brio");
      watch =
        systemd.services."usb-camera-holders-relayed@" or (throw "no watch records a relayed camera");
      card = "MX Brio 705 for Business";
      shell = (account shared).jitsusama.quickshell;
      shown = shell.margin.pieces.camera.holders;
      # The relay's own input, as its service gives it.
      read = builtins.match "[^']*-i '([^']*)'[^']*" relay.script;
      input =
        if read == null then throw "the relay's input can't be read from its script" else lib.head read;
      wiring = {
        "the relay isn't bound to the Brio's capture node" =
          lib.elem "dev-mx\\x2dbrio.device" relay.bindsTo;
        "the relay doesn't take its loopback under the Brio's name" =
          builtins.match ".*/bin/camera-loopback '${card}' /run/mx-brio/device -b 4 -x 1" relay.preStart
          != null;
        "the relay doesn't read the Brio's NV12 at 1080p and 30 frames a second first" =
          lib.hasInfix ''caps="video/x-raw,format=NV12,width=1920,height=1080,framerate=30/1;image/jpeg,'' input;
        "the shell always lists a camera that's there only while the Brio is plugged in" =
          !(lib.elem card shown || lib.elem "mx-brio" shown);
        "the launcher doesn't know the Brio's loopback by its card" =
          (shell.launcher.providers.cameras.cards.${card} or null) == card;
        "the loopback's holders file isn't named as the Brio is" =
          lib.hasSuffix "'/run/camera-holders/${card}'" holders.serviceConfig.ExecStart;
        "the loopback's holders file stays once the Brio has gone" =
          lib.any (lib.hasInfix "/bin/rm -f '/run/camera-holders/${card}'") holders.serviceConfig.ExecStopPost;
        "the Brio's own watch doesn't write in the hidden folder" =
          lib.hasSuffix " /run/camera-holders/.relayed" watch.serviceConfig.ExecStart;
        "a relayed camera's watch could write beside its loopback's" =
          lib.hasInfix ''ENV{CAMERA_RELAYED}!="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="usb-camera-holders@%k.service"'' services.udev.extraRules;
      };
      wrong = lib.attrNames (lib.filterAttrs (_: right: !right) wiring);
      tools = {
        nativeBuildInputs = [
          pkgs.systemd
          pkgs.gst_all_1.gstreamer
        ];
        inherit (relay.environment) GST_PLUGIN_PATH;
      };
    in
    assert lib.assertMsg (wrong == [ ]) (lib.concatStringsSep "; " wrong);
    pkgs.runCommandLocal "the-mx-brio-is-shared" tools ''
      bash ${./tests/the-brio-is-kept-from-the-seat.sh} ${environment.etc."udev/rules.d".source}
      # The relay's plugins alone, as its service has them.
      unset GST_PLUGIN_SYSTEM_PATH_1_0
      bash ${./tests/the-relay-takes-either-format.sh} ${lib.escapeShellArg input}
      touch $out
    '';
}
