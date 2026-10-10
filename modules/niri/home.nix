# Joel's niri: config.kdl says how it behaves, and theme.kdl, written here from
# jitsusama.theme, how it looks. config.kdl is text a machine can add to, with
# lib.mkAfter, such as a panel's scale; niri lets a later setting override an
# earlier one, so the machine's lines win.
#
# The cursor is Adwaita's, as Omarchy's is through its system default. niri
# draws it and tells every program it starts which theme and size to use, and
# its default names a theme called "default" that nothing installs, so without
# this only niri's built-in arrow shows and every other shape (resize, grab,
# text, not-allowed) falls back to it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles shape motion;

  # Anything a gesture can carry rides a spring, which keeps a swipe's speed;
  # the rest runs on the curve.
  spring = ''
    spring damping-ratio=${motion.spring.damping} stiffness=${toString motion.spring.stiffness} epsilon=${motion.spring.epsilon}
  '';
  timed = milliseconds: ''
    duration-ms ${toString milliseconds}
    curve "cubic-bezier" ${lib.concatStringsSep " " motion.curve}
  '';
  animations = {
    workspace-switch = spring;
    horizontal-view-movement = spring;
    window-movement = spring;
    window-resize = spring;
    overview-open-close = spring;
    window-open = timed motion.medium;
    window-close = timed motion.short;
    screenshot-ui-open = timed motion.short;
    config-notification-open-close = timed motion.medium;
    exit-confirmation-open-close = timed motion.medium;
  };
  cursor = {
    name = "Adwaita";
    package = pkgs.adwaita-icon-theme;
    size = 24;
  };
in
{
  imports = [ ../theme/home.nix ];

  # Also links it as ~/.icons/default and names it to GTK, for programs that
  # look there rather than at niri's variables. Home Manager offers this on
  # Linux only, and niri runs nowhere else.
  home.pointerCursor = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    cursor
    // {
      enable = true;
      gtk.enable = true;
    }
  );

  xdg.configFile."niri/config.kdl".text = builtins.readFile ./config.kdl;

  xdg.configFile."niri/theme.kdl".text = ''
    layout {
        gaps ${toString shape.gap}
        background-color "${roles.background}"
        border {
            width ${toString shape.border}
            active-color "${roles.accent}"
            inactive-color "${roles.rule}"
        }
        tab-indicator {
            active-color "${roles.accent}"
            inactive-color "${roles.raised}"
        }
        insert-hint {
            color "${roles.accent}80"
        }
    }
    window-rule {
        geometry-corner-radius ${toString shape.radius}
        clip-to-geometry true
    }
    overview {
        backdrop-color "${roles.deepest}"
    }
    cursor {
        xcursor-theme "${cursor.name}"
        xcursor-size ${toString cursor.size}
    }
    animations {
        slowdown ${motion.slowdown}
    ${
      lib.concatStrings (
        lib.mapAttrsToList (name: body: ''
          ${name} {
          ${body}}
        '') animations
      )
    }}
  '';
}
