# The loopback camera module, for a relay that reads a camera and hands its
# frames on through a camera other programs can open. Whichever module makes
# a loopback turns it on, and takes v4l2loopback-ctl from its package, so the
# module loaded and the program making its cameras are the same build.
#
# Nothing says which program has a loopback camera open: PipeWire doesn't
# offer these cameras, and a relay hears only that a reader came or went.
# So for each camera in holders, a service watches its node, and on every
# open and close writes the programs holding it to a file a shell can watch,
# one name a line and empty while none does.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.v4l2loopback;
  holders = pkgs.writers.writePython3Bin "loopback-holders" { } (builtins.readFile ./holders.py);
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
            output = lib.mkOption {
              type = lib.types.str;
              description = "The file naming the programs that hold it open.";
            };
            unit = lib.mkOption {
              type = lib.types.str;
              description = "The service that makes the camera, whose life this one shares.";
            };
            ignore = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "Programs left out, such as the relay that fills the camera.";
            };
          };
        }
      );
      default = { };
      example = lib.literalExpression ''
        {
          camera = {
            device = "/run/camera/device";
            output = "/run/camera/in-use";
            unit = "camera.service";
            ignore = [ "v4l2-relayd" ];
          };
        }
      '';
      description = "Loopback cameras to say the holders of, by a name for each.";
    };
  };

  config = lib.mkIf cfg.enable {
    boot.extraModulePackages = [ cfg.package ];
    # Each relay makes the loopback it needs, so the module makes none.
    boot.extraModprobeConfig = ''
      options v4l2loopback devices=0
    '';

    systemd.services = lib.mapAttrs' (
      name: camera:
      lib.nameValuePair "${name}-holders" {
        description = "Says which programs have ${name} open";
        # It starts once the camera's service has made the loopback, and
        # stops and starts with it, since each start makes a new one.
        bindsTo = [ camera.unit ];
        after = [ camera.unit ];
        wantedBy = [ camera.unit ];
        serviceConfig = {
          ExecStart = lib.escapeShellArgs (
            [
              (lib.getExe holders)
              camera.device
              camera.output
            ]
            ++ camera.ignore
          );
          Restart = "on-failure";
          RestartSec = 1;
          # It lists every process's open files, which takes reading past
          # their owners' permissions and root's ptrace reading, and writes
          # only beside its output.
          CapabilityBoundingSet = [
            "CAP_DAC_READ_SEARCH"
            "CAP_SYS_PTRACE"
          ];
          NoNewPrivileges = true;
          PrivateNetwork = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          ReadWritePaths = [ (dirOf camera.output) ];
        };
      }
    ) cfg.holders;
  };
}
