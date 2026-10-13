# Pictures and videos from the camera, through `camera picture` or `camera
# video`, which the launcher's Capture collection runs. Both open a
# viewfinder in mpv (camera.lua) that's driven by the keys: Return takes a
# picture, or starts and stops a video, Tab changes which, and Escape closes
# it. Pictures go to Pictures/Camera and are copied, and videos to
# Videos/Camera, beside the screen's own.
#
# The viewfinder plays the camera with nothing buffered and nothing timed,
# so it keeps up with Joel, and mirrored, as a mirror would show him; what's
# saved is the way others see him. The camera is the machine's, from
# jitsusama.camera.device (nixos.nix), or set here for one Home Manager runs
# alone on.
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for its camera.
  osConfig ? { },
  ...
}:
let
  inherit (config.jitsusama.theme) roles font;
  inherit (config.jitsusama.camera) device;
  nerd = import ../theme/nerd.nix { inherit lib; };

  # The theme's look for the viewfinder's panel, as mpv's script options.
  look = lib.concatStringsSep "," (
    lib.mapAttrsToList (name: value: "camera-${name}=${value}") {
      mono = font.mono.family;
      inherit (roles)
        deepest
        shadow
        edge
        text
        muted
        strong
        action
        alert
        ;
      opacity = toString config.jitsusama.theme.floating.opacity;
      picture = nerd "f0100";
      video = config.jitsusama.theme.glyphs.recording.glyph;
    }
  );

  camera = pkgs.writeShellApplication {
    name = "camera";
    runtimeInputs = [
      pkgs.mpv
      pkgs.ffmpeg
      pkgs.wl-clipboard
      pkgs.libnotify
      pkgs.xdg-utils
      pkgs.coreutils
    ];
    text = ''
      mode=''${1:-picture}
      case $mode in
          picture | video) ;;
          *) echo "usage: camera [picture | video]" >&2; exit 2 ;;
      esac
      device=${lib.escapeShellArg device}
      # A camera made at boot is named by a file.
      [ -c "$device" ] || device=$(cat "$device")
      # Drawn on the GPU through Wayland, or in shared memory where there's
      # no GPU to draw with. It idles between sources, so the camera can pass
      # to a recording and back (camera.lua).
      exec mpv "av://v4l2:$device" \
          --vo=gpu-next,wlshm --gpu-context=wayland \
          --title=Camera --wayland-app-id=camera --force-window=immediate \
          --profile=low-latency --untimed --cache=no --hwdec=no --aid=no \
          --vf=hflip --osc=no --input-default-bindings=no \
          --keep-open=no --idle=yes --save-position-on-quit=no --resume-playback=no \
          --script=${./camera.lua} \
          --script-opts=${lib.escapeShellArg look},camera-mode="$mode",camera-device="$device"
    '';
  };
in
{
  imports = [ ../theme/home.nix ];

  options.jitsusama.camera.device = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = osConfig.jitsusama.camera.device or null;
    defaultText = lib.literalExpression "osConfig.jitsusama.camera.device or null";
    description = ''
      The camera pictures and videos are taken with, as a V4L2 device, or a
      file naming one. Null for none, which leaves the camera program out.
    '';
  };

  config.home.packages = lib.optional (device != null) camera;
}
