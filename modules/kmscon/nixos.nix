# The consoles a person signs in on, from Ctrl+Alt+F2 up, drawn by kmscon
# rather than the kernel, in the theme the machine's own account wears. The
# kernel's console draws a bitmap of at most 512 glyphs, so the prompt's
# marks fall back to ASCII stand-ins there, and it can't keep its text off
# the screen's edge. kmscon draws the theme's monospace face with Pango, every
# glyph the face has, at the size the terminal draws it, with the
# terminal's padding round it and its sixteen colours, text and background.
# The kernel's console still shows the boot, the disk's prompt and a failed
# boot's shell (../console/nixos.nix), and the greeter keeps the first.
#
# Its keys are the terminal's (modules/kitty/keys.nix): Shift with Page Up
# and Down scrolls a page, and Ctrl+Shift with equals and minus makes the
# text larger and smaller. kmscon's turning of the screen with Super is
# switched off, since nothing here is ever turned. A held key repeats as
# everywhere else (../keyboard/repeat.nix).
#
# kmscon gains a padding of its own (padding.patch), and its cell becomes
# the face's advance, rounded up, rather than a pixel wider, so letters sit
# as close as they do in the terminal. Should kmscon fail on a console, its
# unit starts the kernel's login prompt there instead, so a console always
# asks. A machine without such an account, or without a theme for it, keeps
# the kernel's console at every login.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.kmscon;

  account = if options ? jitsusama.login.account then config.jitsusama.login.account else null;
  home =
    if account != null && options ? home-manager then
      config.home-manager.users.${account} or null
    else
      null;
  theme = lib.attrByPath [ "jitsusama" "theme" ] null home;

  repeat = import ../keyboard/repeat.nix;

  # Device pixels for a length in logical ones, as the desktop draws it.
  device = pixels: builtins.floor (pixels * cfg.scale + 0.5);

  # The face's size in points, 72 to the inch, as pixels, 96 to the inch.
  size = device (builtins.fromJSON theme.font.mono.size * 4 / 3);

  # A colour as kmscon writes one: red, green and blue, in decimal.
  rgb =
    colour:
    lib.concatMapStringsSep ", " (at: toString (lib.fromHexString (lib.substring at 2 colour))) [
      1
      3
      5
    ];

  # kmscon's names for the sixteen, against the theme's: white is its light
  # grey and bright white its white.
  palette = {
    black = "black";
    red = "red";
    green = "green";
    yellow = "yellow";
    blue = "blue";
    magenta = "magenta";
    cyan = "cyan";
    light-grey = "white";
    dark-grey = "bright_black";
    light-red = "bright_red";
    light-green = "bright_green";
    light-yellow = "bright_yellow";
    light-blue = "bright_blue";
    light-magenta = "bright_magenta";
    light-cyan = "bright_cyan";
    white = "bright_white";
    foreground = "foreground";
    background = "background";
  };

  package = pkgs.kmscon.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [ ./padding.patch ];
  });
in
{
  options.jitsusama.kmscon.scale = lib.mkOption {
    type = lib.types.numbers.positive;
    default = 1;
    example = 2;
    description = ''
      How many device pixels the screen draws for each logical one, as the
      desktop scales it. kmscon draws in device pixels, so its text and
      padding are this many times the theme's.
    '';
  };

  config = lib.mkIf (theme != null) {
    services.kmscon = {
      enable = true;
      inherit package;
      config = {
        # Pango, which sizes the face by its em, as the terminal does, and
        # finds a glyph the face lacks in another. Left to choose, kmscon
        # takes FreeType, which fits the whole line in the size.
        font-engine = "pango";
        font-name = theme.font.mono.family;
        font-size = size;
        # As the terminal pads its text: wider at the sides.
        padding-x = device theme.space.xl;
        padding-y = device theme.space.l;
        palette = "custom";
        # As many lines as the terminal keeps.
        sb-size = 10000;
        xkb-repeat-delay = repeat.delay;
        xkb-repeat-rate = 1000 / repeat.rate;
        # A grab matches the symbol a key makes with its modifiers held, so
        # Ctrl+Shift+equals is plus and Ctrl+Shift+minus is underscore.
        grab-zoom-in = "<Ctrl><Shift>plus";
        grab-zoom-out = "<Ctrl><Shift>underscore";
        grab-rotate-cw = "";
        grab-rotate-ccw = "";
      }
      // lib.mapAttrs' (
        name: role: lib.nameValuePair "palette-${name}" (rgb theme.terminal.${role})
      ) palette;
    };

    # kmscon's unit falls back to getty@ on its console, which NixOS's
    # kmscon takes away, so it's put back, without its claim to be what
    # logind starts on a console switched to, which stays kmscon's.
    systemd.suppressedSystemUnits = lib.mkForce [ ];
    systemd.services."getty@".aliases = lib.mkForce [ ];
  };
}
