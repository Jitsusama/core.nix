# A machine's IR camera, as a camera any program can open: ir-camera
# (src/main.rs) relays its raw greyscale frames to a loopback camera,
# /dev/ir-camera, as 8-bit grey, and only while a program reads it, so the
# camera and its IR light are off otherwise. The hardware's module says
# where the raw frames come from, and what wires a media pipeline to them.
#
# Nothing here signs anyone in. Howdy, should a machine turn it on, reads
# this camera, and PAM then offers it only to the services that ask for it,
# as enough on its own, since a face is weaker than a password.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.irCamera;
  relay = pkgs.callPackage ./package.nix { };
  loopback = lib.getExe' config.boot.kernelPackages.v4l2loopback.bin "v4l2loopback-ctl";
  name = "IR Camera";
  sink = "/run/ir-camera-loopback/sink";

  # The loopback announces itself as an output as well as a camera, which
  # browsers take as not a camera, so it's never offered for a call.
  add = pkgs.writeShellScript "ir-camera-add" ''
    set -eu
    device=$(${loopback} add -n '${name}' -x 0 -b 2)
    ln -sfn "$device" ${sink}
  '';
  delete = pkgs.writeShellScript "ir-camera-delete" ''
    if device=$(readlink ${sink}); then
      ${loopback} delete "$device"
    fi
  '';

  # Over Howdy's own defaults, and under a machine's.
  overHowdy = lib.mkOverride 900;
in
{
  options.jitsusama.irCamera = {
    enable = lib.mkEnableOption "the IR camera, as a camera programs can open";
    source = lib.mkOption {
      type = lib.types.str;
      example = "/run/ir-camera/source";
      description = ''
        The raw camera to read: a V4L2 capture node, or a link the prepare
        program makes to one.
      '';
    };
    format = lib.mkOption {
      type = lib.types.enum [
        "Y10"
        "Y12"
        "Y16"
      ];
      description = "The raw camera's V4L2 format.";
    };
    width = lib.mkOption {
      type = lib.types.ints.positive;
      description = "The raw camera's width, in pixels.";
    };
    height = lib.mkOption {
      type = lib.types.ints.positive;
      description = "The raw camera's height, in pixels.";
    };
    prepare = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        A program run before each stream, as the relay's user, who is in the
        video group: one that wires a media pipeline to the camera, and may
        link the source to the node the pipeline ends at. Null for none.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    boot.extraModulePackages = [ config.boot.kernelPackages.v4l2loopback ];
    # The relay makes its own loopback, so the module makes none.
    boot.extraModprobeConfig = ''
      options v4l2loopback devices=0
    '';

    services.udev.extraRules = ''
      SUBSYSTEM=="video4linux", ATTR{name}=="${name}", SYMLINK+="ir-camera"
    '';

    # Nor is it offered through PipeWire.
    services.pipewire.wireplumber.extraConfig."50-hide-ir-camera"."monitor.v4l2.rules" = [
      {
        matches = [ { "device.product.name" = name; } ];
        actions.update-props."device.disabled" = true;
      }
    ];

    services.howdy.settings.video.device_path = overHowdy "/dev/ir-camera";
    services.howdy.control = lib.mkDefault "sufficient";
    security.pam.howdy.enable = lib.mkDefault false;

    # Making the loopback needs root, so it's a unit of its own, and the
    # relay's, which needs none, can be closed off. The loopback outlasts a
    # relay that fails, so the camera programs open stays where it is.
    systemd.services.ir-camera-loopback = {
      description = "The loopback camera the IR camera is relayed to";
      wants = [ "modprobe@v4l2loopback.service" ];
      after = [ "modprobe@v4l2loopback.service" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        RuntimeDirectory = "ir-camera-loopback";
        ExecStart = add;
        ExecStop = delete;
      };
    };

    systemd.services.ir-camera = {
      description = "The IR camera, as a V4L2 camera programs can open";
      wantedBy = [ "multi-user.target" ];
      bindsTo = [ "ir-camera-loopback.service" ];
      after = [ "ir-camera-loopback.service" ];
      serviceConfig = {
        ExecStart = lib.escapeShellArgs (
          [
            (lib.getExe relay)
            "--source"
            cfg.source
            "--sink"
            sink
            "--format"
            cfg.format
            "--width"
            (toString cfg.width)
            "--height"
            (toString cfg.height)
          ]
          ++ lib.optionals (cfg.prepare != null) [
            "--prepare"
            cfg.prepare
          ]
        );
        Restart = "on-failure";
        RestartSec = 2;
        RuntimeDirectory = "ir-camera";
        DynamicUser = true;
        SupplementaryGroups = [ "video" ];
        DevicePolicy = "closed";
        DeviceAllow = [
          "char-video4linux rw"
          "char-media rw"
        ];
        CapabilityBoundingSet = "";
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        PrivateNetwork = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        RestrictAddressFamilies = "none";
        RestrictNamespaces = true;
        RestrictRealtime = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" ];
      };
    };
  };
}
