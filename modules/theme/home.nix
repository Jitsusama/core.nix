# The look every program Joel sees shares, read from one theme file beside
# this module (docs/design.md says what a theme holds and why). A program's
# module reads it from jitsusama.theme and writes it in the program's own
# format, so a terminal, a notification and the lock screen all look alike,
# and picking another theme changes them all. Surfaces name what a colour
# means (roles), never the colour itself, and the theme is checked for
# legibility as the machine is evaluated.
{ config, lib, ... }:
let
  cfg = config.jitsusama.theme;
  contrast = import ./contrast.nix { inherit lib; };

  themes = {
    osaka-jade = ./osaka-jade.toml;
  };

  theme = builtins.fromTOML (builtins.readFile cfg.file);

  color = lib.types.strMatching "#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?";

  pigment =
    name:
    cfg.palette.${name} or (throw "The theme names a pigment, ${name}, that its palette doesn't have.");

  colorOption =
    pick: description:
    lib.mkOption {
      type = color;
      default = pigment pick;
      defaultText = lib.literalMD "the pigment the theme file names";
      inherit description;
    };

  roles = {
    background = "behind everything: windows, the terminal, menus";
    sunken = "below the background: a sidebar, an inactive tab, a list";
    deepest = "the very back: the overview's backdrop, a tab bar";
    raised = "above the background: a hovered row, an indicator";
    selection = "behind what's selected";
    text = "what Joel reads and acts on";
    strong = "text that stands out: a heading, a selected row, the cursor";
    muted = "text that gives context: labels, units, hints";
    rule = "separators and marks that aren't read as text";
    accent = "where Joel's attention is: one focal point per screen state";
    on_accent = "text drawn on the accent or on any of the state colours";
    alert = "something that needs Joel: a failure, an urgent window";
    warning = "something that will need him soon";
    success = "something that went well";
  };

  terminal = {
    background = "behind the terminal's text";
    foreground = "the terminal's text";
    cursor = "the terminal's cursor";
    cursor_text = "text under the terminal's cursor";
    selection_background = "behind selected text in the terminal";
    selection_foreground = "selected text in the terminal";
  }
  // lib.genAttrs ansi (name: "ANSI colour ${name}, which programs in the terminal draw with");

  ansi = [
    "black"
    "red"
    "green"
    "yellow"
    "blue"
    "magenta"
    "cyan"
    "white"
    "bright_black"
    "bright_red"
    "bright_green"
    "bright_yellow"
    "bright_blue"
    "bright_magenta"
    "bright_cyan"
    "bright_white"
  ];

  # The pairs the design draws text in, and the least each may measure:
  # 4.5:1 for text, as WCAG asks of body text. A role pair not listed here
  # isn't one a surface draws text in, such as muted on the selection, where
  # hints switch to text instead.
  readable =
    let
      on =
        backgrounds: foregrounds:
        lib.cartesianProduct {
          fg = foregrounds;
          bg = backgrounds;
        };
      role = name: {
        inherit name;
        value = cfg.roles.${name};
      };
      term = name: {
        name = "terminal ${name}";
        value = cfg.terminal.${name};
      };
    in
    on
      (map role [
        "background"
        "sunken"
        "deepest"
      ])
      (
        map role [
          "text"
          "strong"
          "muted"
          "accent"
          "alert"
          "warning"
          "success"
        ]
      )
    ++
      on
        (map role [
          "raised"
          "selection"
        ])
        (
          map role [
            "text"
            "strong"
          ]
        )
    ++ on (map role [
      "accent"
      "alert"
      "warning"
      "success"
    ]) [ (role "on_accent") ]
    ++ on [ (term "background") ] (map term (lib.remove "black" ansi ++ [ "foreground" ]))
    ++ on [ (term "selection_background") ] [ (term "selection_foreground") ];

  illegible = lib.filter (pair: contrast.ratio pair.fg.value pair.bg.value < 4.5) readable;
in
{
  options.jitsusama.theme = {
    name = lib.mkOption {
      type = lib.types.enum (lib.attrNames themes);
      default = "osaka-jade";
      description = "Which of core's themes to draw everything in.";
    };

    file = lib.mkOption {
      type = lib.types.path;
      default = themes.${cfg.name};
      defaultText = lib.literalMD "the file `jitsusama.theme.name` names";
      description = ''
        The theme file to draw everything in, for a theme of a machine
        repository's own. docs/design.md describes the format.
      '';
    };

    palette = lib.mkOption {
      type = lib.types.attrsOf color;
      default = theme.palette;
      defaultText = lib.literalMD "the theme file's palette";
      description = ''
        Every colour the theme uses, by the name the theme gives it. Change one
        here and everything that means it changes.
      '';
    };

    roles = lib.mapAttrs (
      name: description: colorOption theme.roles.${name} "The colour for ${description}."
    ) roles;

    terminal = lib.mapAttrs (
      name: description: colorOption theme.terminal.${name} "The colour for ${description}."
    ) terminal;

    shades = lib.mkOption {
      type = lib.types.listOf color;
      default = map pigment theme.shades;
      defaultText = lib.literalMD "the theme file's shades";
      description = ''
        Eight colours from the background to the brightest text, for whatever
        draws in shades, such as a graph.
      '';
    };

    font = {
      mono = {
        family = lib.mkOption {
          type = lib.types.str;
          default = "MonaspiceXe Nerd Font";
          description = ''
            The monospaced face, for where characters line up or the content is
            code: the terminal, the editor, chords, paths and changing numbers.
          '';
        };
        size = lib.mkOption {
          # A string, since Nix prints the number 10.5 as 10.500000.
          type = lib.types.strMatching "[0-9]+(\\.[0-9]+)?";
          # WezTerm's 14 on macOS: macOS counts 72 points per inch and Linux
          # 96, so the same size is a smaller number here.
          default = "10.5";
          description = "The monospaced face's size in points, as Linux counts them.";
        };
      };
      sans = {
        family = lib.mkOption {
          type = lib.types.str;
          # Drawn with Monaspace by GitHub Next, so the two share proportions.
          default = "Mona Sans";
          description = ''
            The proportional face, for where people read: interfaces, menus,
            notifications, dialogs and the greeter.
          '';
        };
        size = lib.mkOption {
          type = lib.types.strMatching "[0-9]+(\\.[0-9]+)?";
          default = "10.5";
          description = "The proportional face's size in points, as Linux counts them.";
        };
      };
    };

    shape = {
      border = lib.mkOption {
        type = lib.types.ints.unsigned;
        default = 2;
        description = "How wide, in pixels, the border around a window or a prompt is.";
      };
      radius = lib.mkOption {
        type = lib.types.ints.unsigned;
        # Square, as Omarchy draws them.
        default = 0;
        description = "How round, in pixels, the corners of a window or a prompt are.";
      };
      gap = lib.mkOption {
        type = lib.types.ints.unsigned;
        default = 10;
        description = "The space, in pixels, between windows and around the text in a prompt.";
      };
    };
  };

  config.assertions = [
    {
      assertion = lib.length cfg.shades == 8;
      message = "The theme's shades have to be eight colours, from the background to the brightest text.";
    }
    {
      assertion = illegible == [ ];
      message = ''
        The theme draws text too faint to read. Each pair needs 4.5:1:
        ${lib.concatMapStringsSep "\n" (
          pair:
          "  ${pair.fg.name} on ${pair.bg.name}: ${contrast.show (contrast.ratio pair.fg.value pair.bg.value)}:1"
        ) illegible}
      '';
    }
  ];
}
