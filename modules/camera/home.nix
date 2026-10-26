# Pictures and videos from the camera, through `camera picture` or `camera
# video`, which the launcher's Capture collection runs. Both open a
# viewfinder in mpv (camera.lua) that's driven by the keys: Return takes a
# picture, or starts and then stops a video, and the viewfinder goes once
# it's done; Tab changes which, and Escape closes it. Pictures go to Pictures/Camera and are copied, and videos to
# Videos/Camera, beside the screen's own.
#
# The viewfinder plays the camera with nothing buffered and nothing timed,
# so it keeps up with Joel, and mirrored, as a mirror would show him; what's
# saved is the way others see him. The camera is one plugged in while
# there is one (camera.sh), and otherwise the machine's, from
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
      sans = font.sans.family;
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
    text = lib.concatLines [
      "device=${lib.escapeShellArg device} look=${lib.escapeShellArg look} script=${./camera.lua}"
      (builtins.readFile ./camera.sh)
    ];
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
