# Logitech's MX Brio, shared by every program at once. A USB camera streams
# to one program at a time, and Meet tries a camera only as Joel joins, so
# whichever program held the Brio first, Tuple or a test, cost him his
# picture for the whole call. So a relay alone opens the Brio, and plays it
# into a loopback camera any number of programs can read together, named
# as the Brio is, so every program's list, the shell and the desk light
# read as they did before.
#
# The relay passes the Brio's own NV12 through untouched, rather than its
# MJPEG decoded: the Brio sends NV12 at 1080p and 30 frames a second over
# USB 3, which spares decoding a frame in 6 to 22 ms of a core in software
# and about 30 ms on the GPU, and spares Chrome its own decode too. Tuple
# reads NV12 as well, through PipeWire. The relay opens the Brio only while
# a program reads the loopback, so the Brio streams, and its light shows,
# only then.
#
# The Brio itself is kept from Joel's seat, so no program but the relay
# opens it: Chrome lists only the loopback, and PipeWire, run as him,
# offers only the loopback too. Its own watch (../usb-camera-holders) goes
# on recording how it streams, in a hidden folder, while the loopback's
# holders say who's on camera. A switch that brings the sharing in takes
# hold of a Brio already plugged in, so nobody plugs it in again
# (take-hold.sh).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  card = "MX Brio 705 for Business";
  device = "/run/mx-brio/device";
  relay = pkgs.callPackage ../v4l2loopback/relay.nix { };
  ctl = config.jitsusama.v4l2loopback.package.bin;
  takeLoopback = pkgs.callPackage ../v4l2loopback/loopback.nix { inherit ctl; };
  plugins = lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" [
    pkgs.gst_all_1.gstreamer.out
    pkgs.gst_all_1.gst-plugins-base
    pkgs.gst_all_1.gst-plugins-good
  ];

  # One format end to end, so nothing converts or scales: the Brio's NV12
  # at 1080p, the most a call sends, passed through untouched. Only a USB 2
  # link, too narrow for it, takes the Brio's MJPEG instead, decoded here,
  # so the Brio, hidden from everything else, never leaves Joel with no
  # camera. A leaky queue drops a stale frame rather than delaying the next.
  caps = "video/x-raw,format=NV12,width=1920,height=1080,framerate=30/1";
  # Either picture is a choice of two caps, which GStreamer reads only as
  # a quoted value; bare, its parser takes the first for an element.
  input = lib.concatStringsSep " ! " [
    "v4l2src device=/dev/mx-brio"
    ''capsfilter caps="${caps};image/jpeg,width=1920,height=1080,framerate=30/1"''
    "decodebin3"
    "videoconvert"
    caps
  ];
  output = lib.concatStringsSep " ! " [
    "appsrc name=appsrc caps=${caps}"
    "queue leaky=downstream max-size-buffers=3"
    # As the built-in camera's relay has it, so the loopback's sink can
    # meet the picture however it asks; NV12 already, it passes untouched.
    "videoconvert"
    "v4l2sink name=v4l2sink device=$(cat ${device}) sync=false"
  ];

  # After systemd's 70-uaccess.rules, which hands every camera to whoever
  # sits at the seat, and before its 73-seat-late.rules, which does the
  # handing, so the tag is gone by then; logind, as a session starts, hands
  # only nodes still tagged. That keeps the Brio from Joel's programs and
  # PipeWire, which runs as him, because the video group, which udev gives
  # every camera, isn't his. Every node of the Brio is marked
  # relayed, for its watch to write apart; its capture node, the first,
  # gets a fixed name, for the relay to open and be bound to.
  rules = pkgs.writeTextDir "lib/udev/rules.d/71-mx-brio.rules" ''
    SUBSYSTEM=="video4linux", ENV{ID_USB_DRIVER}=="uvcvideo", ENV{ID_VENDOR_ID}=="046d", ENV{ID_MODEL_ID}=="091d", TAG-="uaccess", ENV{CAMERA_RELAYED}="1"
    SUBSYSTEM=="video4linux", ENV{CAMERA_RELAYED}=="1", ATTR{index}=="0", SYMLINK+="mx-brio", TAG+="systemd", ENV{SYSTEMD_WANTS}+="mx-brio.service"
  '';
in
{
  imports = [
    ../usb-camera-holders/nixos.nix
    ../v4l2loopback/nixos.nix
  ];

  # The relay is packaged for x86 machines alone, so elsewhere the Brio
  # stays as it was, the seat's to open directly, one program at a time.
  config = lib.mkIf (lib.meta.availableOn pkgs.stdenv.hostPlatform pkgs.v4l2-relayd) {
    services.udev.packages = [ rules ];

    # Hiding the Brio rests on its users not being in the video group, so a
    # machine that puts one there is told, rather than finding a program
    # holding the Brio itself and the loopback left dark.
    warnings =
      let
        inVideo = lib.attrNames (
          lib.filterAttrs (
            name: user:
            lib.elem "video" user.extraGroups || lib.elem name (config.users.groups.video.members or [ ])
          ) (lib.filterAttrs (_: user: user.isNormalUser) config.users.users)
        );
      in
      lib.optional (inVideo != [ ]) (
        "The MX Brio is kept from the seat, but ${lib.concatStringsSep ", " inVideo} can still open it"
        + " through the video group, beside its loopback."
      );

    # Who's on camera, from the loopback, under the Brio's own name.
    jitsusama.v4l2loopback.holders.mx-brio = {
      inherit device card;
      file = card;
      comesAndGoes = true;
      unit = "mx-brio.service";
      ignoreUnits = [ "mx-brio.service" ];
    };

    # A switch starts it as it's new, and again whenever the rules change. It
    # runs at every boot too, where udev's first look has already done this:
    # it finds the Brio held, or none, and waits only on the Brio's events.
    systemd.services.mx-brio-take-hold = {
      description = "Take hold of an MX Brio already plugged in";
      wantedBy = [ "multi-user.target" ];
      after = [
        "systemd-udevd.service"
        "systemd-udev-trigger.service"
      ];
      restartTriggers = [ rules ];
      path = [
        pkgs.acl
        # The running systemd's udevadm, which speaks to its own udevd.
        config.systemd.package
      ];
      script = builtins.readFile ./take-hold.sh;
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        # A switch waits on it, and udevadm's settling has no bound of its
        # own; a replay takes seconds, so a minute is a wedged udev.
        TimeoutStartSec = "1min";
      };
    };

    systemd.services.mx-brio = {
      description = "The MX Brio, as a camera every program can open at once";
      # It goes when the Brio does, and starts again as it's plugged in.
      bindsTo = [ "dev-mx\\x2dbrio.device" ];
      after = [
        "dev-mx\\x2dbrio.device"
        "modprobe@v4l2loopback.service"
      ];
      wants = [ "modprobe@v4l2loopback.service" ];
      environment.GST_PLUGIN_PATH = plugins;
      serviceConfig = {
        RuntimeDirectory = "mx-brio";
        # It runs as root to make the loopback camera, as the built-in
        # camera's relay does, and needs no network and no shared /tmp.
        PrivateNetwork = true;
        PrivateTmp = true;
        Restart = "always";
        RestartSec = 2;
        # A call's picture is as live as its sound, so the relay comes first
        # for the processor and the disk, and never waits on swap.
        CPUWeight = 1000;
        IOWeight = 1000;
        MemorySwapMax = 0;
      };
      # Four buffers keep the frame rate up, and exclusive capture lets Chrome
      # list it as a camera. A loopback left from before is taken rather than
      # added beside (../v4l2loopback/loopback.sh says why).
      preStart = "${lib.getExe takeLoopback} ${lib.escapeShellArg card} ${device} -b 4 -x 1";
      script = ''
        exec ${lib.getExe relay} -i ${lib.escapeShellArg input} -o "${output}"
      '';
      postStop = ''${lib.getExe' ctl "v4l2loopback-ctl"} delete "$(cat ${device})" || true'';
    };
  };
}
