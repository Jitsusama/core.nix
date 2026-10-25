# The loopback camera module, for a relay that reads a camera and hands its
# frames on through a camera other programs can open. Whichever module makes
# a loopback turns it on, and takes v4l2loopback-ctl from its package, so the
# module loaded and the program making its cameras are the same build.
#
# Nothing says which program has a loopback camera open: PipeWire doesn't
# offer these cameras, and a relay hears only that a reader came or went.
# So for each camera in holders, a service watches its node, and on every
# open and close writes the programs holding it to /run/camera-holders/<name>
# for a shell to watch, one name a line and empty while none does or the
# camera is down. The folder lasts the boot, since a shell's watch on it
# would be lost with it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.v4l2loopback;
  holders = pkgs.callPackage ./holders/package.nix { };
  holdersDir = "/run/camera-holders";
in
{
  options.jitsusama.v4l2loopback = {
    enable = lib.mkEnableOption "the loopback camera module";
    package = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      # A relay stops writing while it waits for its next reader, and the
      # module would hand that reader the last frame written, the end of the
      # session before. Written here; the patch says why.
      default = config.boot.kernelPackages.v4l2loopback.overrideAttrs (old: {
        patches = old.patches or [ ] ++ [ ./reader-starts-after-newest.patch ];
      });
      defaultText = lib.literalMD "the kernel's `v4l2loopback`, patched";
      description = "The module, built for the kernel, with `v4l2loopback-ctl` in its `bin`.";
    };
    holders = lib.mkOption {
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            device = lib.mkOption {
              type = lib.types.str;
              description = "The camera's node, or a file naming it.";
            };
            unit = lib.mkOption {
              type = lib.types.str;
              description = "The service that makes the camera, whose life this one shares.";
            };
            card = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = ''
                The loopback's name, so a node number another camera has
                been given since isn't taken for this one. Null for any.
              '';
            };
            sensor = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "i2c-OVTI08F4:00";
              description = ''
                The I2C device of the sensor the camera is relayed from, as
                /sys/bus/i2c/devices names it, for desk-facts to say what it
                is, which way it faces and whether it's powered, which it
                reads without opening the camera. Null where there's none.
              '';
            };
            ignoreUnits = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = ''
                Units whose processes are left out, such as the relay's that
                fills the camera. By unit rather than by name, since a
                program can take any name but can't join a unit.
              '';
            };
          };
        }
      );
      default = { };
      example = lib.literalExpression ''
        {
          camera = {
            device = "/run/camera/device";
            unit = "camera.service";
            card = "Built-in Camera";
            ignoreUnits = [ "camera.service" ];
          };
        }
      '';
      description = ''
        Loopback cameras to say the holders of, each in
        /run/camera-holders/<name>.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    boot.extraModulePackages = [ cfg.package ];
    # Each relay makes the loopback it needs, so the module makes none.
    boot.extraModprobeConfig = ''
      options v4l2loopback devices=0
    '';

    # The files' folder lasts the boot, so a shell watching it keeps its
    # watch however often a camera's service starts again. USB cameras'
    # holders (../usb-camera-holders) share it, so it's set by name, which
    # the two modules' settings merge on.
    systemd.tmpfiles.settings = lib.mkIf (cfg.holders != { }) {
      camera-holders.${holdersDir}.d = {
        mode = "0755";
        user = "root";
        group = "root";
      };
    };

    systemd.services = lib.mapAttrs' (
      name: camera:
      let
        output = "${holdersDir}/${name}";
      in
      lib.nameValuePair "${name}-holders" {
        description = "Says which programs have ${name} open";
        # It starts once the camera's service has made the loopback, and
        # stops and starts with it, since each start makes a new one.
        bindsTo = [ camera.unit ];
        after = [ camera.unit ];
        wantedBy = [ camera.unit ];
        serviceConfig = {
          ExecStart = lib.escapeShellArgs (
            [ (lib.getExe holders) ]
            ++ lib.optionals (camera.card != null) [
              "--card"
              camera.card
            ]
            ++ lib.concatMap (unit: [
              "--ignore-unit"
              unit
            ]) camera.ignoreUnits
            ++ [
              camera.device
              output
            ]
          );
          # Stopped with its camera, it may not get to say nobody itself.
          ExecStopPost = "${pkgs.coreutils}/bin/truncate -s 0 ${output}";
          Restart = "on-failure";
          RestartSec = 1;
          # It lists every process's open files, which takes reading past
          # their owners' permissions and root's ptrace reading. Root is
          # still root, so everything else it could reach is closed off: it
          # writes only in its folder, and it can't trace, load, tune or
          # talk to anything.
          CapabilityBoundingSet = [
            "CAP_DAC_READ_SEARCH"
            "CAP_SYS_PTRACE"
          ];
          NoNewPrivileges = true;
          PrivateNetwork = true;
          PrivateTmp = true;
          ProtectSystem = "strict";
          ProtectHome = "read-only";
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectKernelLogs = true;
          ProtectControlGroups = true;
          ProtectClock = true;
          ProtectHostname = true;
          RestrictAddressFamilies = "none";
          RestrictNamespaces = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          SystemCallArchitectures = "native";
          SystemCallFilter = [
            "@system-service"
            "~@privileged @resources"
          ];
          ReadWritePaths = [ holdersDir ];
        };
      }
    ) cfg.holders;
  };
}
