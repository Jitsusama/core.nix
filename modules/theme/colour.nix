# The theme's colours: its palette, the roles surfaces draw in, the
# terminal's and code's colours and the shades, each named by what it means,
# and the pairs text is drawn in, held to WCAG and APCA as the machine is
# evaluated (docs/design.md, Colour).
{
  config,
  lib,
  ...
}:
let
  cfg = config.jitsusama.theme;
  contrast = import ./contrast.nix { inherit lib; };
  mix = import ./mix.nix { inherit lib; };
  apca = import ./apca.nix { inherit lib; };
  inherit (import ./shared.nix { inherit lib cfg; }) theme colours;

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

  terminal = {
    background = "behind the terminal's text";
    foreground = "the terminal's text";
    cursor = "the terminal's cursor";
    cursor_text = "text under the terminal's cursor";
    selection_background = "behind selected text in the terminal";
    selection_foreground = "selected text in the terminal";
  }
  // lib.genAttrs ansi (name: "ANSI colour ${name}, which programs in the terminal draw with");

  # Code's roles: what each means, the role it wears where the theme file
  # names no pigment, and how it's set. Structure goes by brightness, the
  # language's own words strong and its punctuation muted; meaning goes by a
  # few hues; comments are written in italics, as handwriting.
  syntax = {
    comment = {
      means = "comments and documentation, written for people";
      role = "muted";
      italic = true;
    };
    keyword = {
      means = "the language's own words: fn, if, return, import";
      role = "strong";
    };
    punctuation = {
      means = "brackets, commas and separators";
      role = "muted";
    };
    operator = {
      means = "operators: = + -> &&";
      role = "text";
    };
    string = {
      means = "strings and characters";
      role = "success";
    };
    constant = {
      means = "numbers, booleans and named constants";
      role = "machine";
    };
    function = {
      means = "functions and methods, where they're defined and called";
      role = "program";
    };
    type = {
      means = "types, traits, classes and their constructors";
      role = "voice";
    };
    special = {
      means = "what the language reads specially: escapes, macros, attributes, patterns";
      role = "person";
    };
  };

  # What the theme file says of a syntax role: a pigment's name, or a table
  # of a pigment and a style.
  written =
    name:
    let
      entry = (theme.syntax or { }).${name} or { };
    in
    if builtins.isString entry then { pigment = entry; } else entry;

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
        value = cfg.colours.${name};
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
          "action"
          "program"
          "machine"
          "person"
          "voice"
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
    # Code is drawn on the background, and in floats on the sunken shade.
    ++
      on
        (map role [
          "background"
          "sunken"
        ])
        (
          lib.mapAttrsToList (name: _: {
            name = "syntax ${name}";
            value = cfg.syntax.${name}.colour;
          }) syntax
        )
    ++ on [ (term "background") ] (map term (lib.subtractLists background ansi ++ [ "foreground" ]))
    ++ on [ (term "selection_background") ] [ (term "selection_foreground") ];

  # The ANSI colours at the background's own end, which programs don't draw
  # text in on that background: black on a dark terminal, white on a light.
  background =
    if cfg.appearance == "light" then
      [
        "white"
        "bright_white"
      ]
    else
      [ "black" ];

  illegible = lib.filter (pair: contrast.ratio pair.fg.value pair.bg.value < 4.5) readable;

  # The text roles on the grounds they're read on, held to APCA as well:
  # WCAG's ratio passes a dark theme's dim text that APCA, weighing how
  # light the background is, finds too dim for reading. Text and strong are
  # body text, which APCA asks Lc 75 of; muted gives context in labels and
  # hints, held to the 60 it asks of any text. A mark's hue is a glyph, not
  # words, held to the 45 APCA asks of a fine mark, a warning's and an
  # alert's as much as any, since a row needing Joel wears them on its mark
  # (VKCF3Z, 2026-11-07: a problem never reads fainter). A row's marks are read
  # on the selection too, as a floating surface draws it: the selection at
  # its opacity over the deepest shade. Its text stays with WCAG alone, since
  # the raised surface and the terminal's selection are drawn translucent
  # over what changes.
  dim =
    let
      grounds = [
        "background"
        "sunken"
        "deepest"
      ];
      held = least: names: {
        inherit least names;
      };
      marks = [
        "action"
        "program"
        "machine"
        "person"
        "voice"
        "alert"
        "warning"
      ];
      selected = mix (builtins.fromJSON cfg.floating.selection) cfg.colours.selection cfg.colours.deepest;
    in
    lib.filter (pair: apca.lightness pair.fg pair.bg < pair.least) (
      map (name: {
        least = 45;
        name = "${name} on a selected row";
        fg = cfg.colours.${name};
        bg = selected;
      }) marks
      ++
        lib.concatMap
          (
            rule:
            lib.concatMap (
              ground:
              map (name: {
                inherit (rule) least;
                name = "${name} on ${ground}";
                fg = cfg.colours.${name};
                bg = cfg.colours.${ground};
              }) rule.names
            ) grounds
          )
          [
            (held 75 [
              "text"
              "strong"
            ])
            (held 60 [ "muted" ])
            (held 45 marks)
          ]
    );
in
{
  options.jitsusama.theme = {
    palette = lib.mkOption {
      type = lib.types.attrsOf color;
      default = theme.palette;
      defaultText = lib.literalMD "the theme file's palette";
      description = ''
        Every colour the theme uses, by the name the theme gives it. Change one
        here and everything that means it changes.
      '';
    };

    colours = lib.mapAttrs (
      name: description: colorOption theme.colours.${name} "The colour for ${description}."
    ) colours;

    terminal = lib.mapAttrs (
      name: description: colorOption theme.terminal.${name} "The colour for ${description}."
    ) terminal;

    syntax = lib.mapAttrs (name: about: {
      colour = lib.mkOption {
        type = color;
        default =
          if written name ? pigment then pigment (written name).pigment else cfg.colours.${about.role};
        defaultText = lib.literalMD "the pigment the theme file names, or the ${about.role} role";
        description = "The colour for ${about.means}.";
      };
      italic = lib.mkOption {
        type = lib.types.bool;
        default = (written name).italic or (about.italic or false);
        defaultText = lib.literalMD "what the theme file says";
        description = "Whether ${about.means} are set in italics.";
      };
      bold = lib.mkOption {
        type = lib.types.bool;
        default = (written name).bold or false;
        defaultText = lib.literalMD "what the theme file says";
        description = "Whether ${about.means} are set in bold.";
      };
    }) syntax;

    shades = lib.mkOption {
      type = lib.types.listOf color;
      default = map pigment theme.shades;
      defaultText = lib.literalMD "the theme file's shades";
      description = ''
        Eight colours from the background to the brightest text, for whatever
        draws in shades, such as a graph.
      '';
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
    {
      assertion = dim == [ ];
      message = ''
        The theme draws text or marks too dim to read by APCA. Text needs Lc 75,
        muted Lc 60 and a mark's hue Lc 45:
        ${lib.concatMapStringsSep "\n" (
          pair: "  ${pair.name}: Lc ${apca.show (apca.lc pair.fg pair.bg)}, under ${toString pair.least}"
        ) dim}
      '';
    }
  ];
}
