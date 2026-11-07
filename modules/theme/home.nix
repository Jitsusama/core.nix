# The look every program Joel sees shares, read from one theme file beside
# this module (docs/design.md says what a theme holds and why). A program's
# module reads it from jitsusama.theme and writes it in the program's own
# format, so a terminal, a notification and the lock screen all look alike,
# and picking another theme changes them all. Surfaces name what a colour
# means (its colours), never the colour itself, and the theme is checked for
# legibility as the machine is evaluated.
{ config, lib, ... }:
let
  cfg = config.jitsusama.theme;
  contrast = import ./contrast.nix { inherit lib; };
  mix = import ./mix.nix { inherit lib; };
  apca = import ./apca.nix { inherit lib; };

  themes = {
    osaka-jade = ./osaka-jade.toml;
    flexoki-light = ./flexoki-light.toml;
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

  colours = {
    background = "behind everything: windows, the terminal, menus";
    sunken = "below the background: a sidebar, an inactive tab, a list";
    deepest = "the very back: the overview's backdrop, a tab bar";
    raised = "above the background: a hovered row, an indicator";
    selection = "behind what's selected";
    text = "what Joel reads and acts on";
    strong = "text that stands out: a heading, a selected row, the cursor";
    muted = "text that gives context: labels, units, hints";
    rule = "separators and marks that aren't read as text";
    edge = "the outline of what isn't focused: a window, a field at rest";
    shadow = "the shadow under a floating surface, drawn faint";
    accent = "where Joel's attention is: one focal point per screen state";
    on_accent = "text drawn on the accent or on any of the state colours";
    alert = "something that needs Joel: a failure, an urgent window";
    warning = "something that will need him soon";
    success = "something that went well";
    # The hues glyphs wear, never text: each is one kind of thing, wherever
    # it's drawn (docs/design.md, Colour).
    action = "a glyph for something to do: a command, a shortcut";
    program = "a glyph for a program: an application, where a notice came from";
    machine = "a glyph for the machine itself: power, security, hardware";
    person = "a glyph for a person: a message, a mention, a call";
    voice = "a glyph for the desktop speaking for itself: an empty list, a greeting";
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

  # Each pair of neighbouring rungs whose outer isn't wider than its inner,
  # which would let two levels of structure share a space.
  widening = lib.filter (pair: cfg.space.rung.${pair.inner} >= cfg.space.rung.${pair.outer}) (
    lib.zipListsWith (inner: outer: {
      inner = inner.name;
      outer = outer.name;
    }) spaceRungs (lib.tail spaceRungs)
  );

  # The text roles on the grounds they're read on, held to APCA as well:
  # WCAG's ratio passes a dark theme's dim text that APCA, weighing how
  # light the background is, finds too dim for reading. Text and strong are
  # body text, which APCA asks Lc 75 of; muted gives context in labels and
  # hints, held to the 60 it asks of any text. A mark's hue is a glyph, not
  # words, held to the 45 APCA asks of a fine mark. A row's marks are read
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

  # A table the theme file may leave out, and a value in it, falling back to
  # this module's default.
  given =
    table: key: default:
    (theme.${table} or { }).${key} or default;

  number = lib.types.strMatching "[0-9]+(\\.[0-9]+)?";

  # Each step of the space scale, in units. One unit apart at the small end,
  # where a pixel shows, and further apart as the steps grow.
  spaceSteps = {
    xs = 1;
    s = 2;
    m = 3;
    l = 4;
    xl = 6;
    xxl = 8;
    xxxl = 12;
    huge = 16;
  };

  # Each rung of the space ladder, in units, inside out: the space one level
  # of structure sits apart by, each wider than the one inside it, so space
  # alone says what belongs together (docs/design.md, Space). Groups take
  # five units, the one rung off the scale, since the ladder needs a rung
  # between things on a line and a frame's own room.
  spaceRungs = [
    {
      name = "tie";
      units = 1;
      between = "the parts of one token, such as a chord's keys";
    }
    {
      name = "mark";
      units = 2;
      between = "a mark and its text, or a key and its word";
    }
    {
      name = "apart";
      units = 3;
      between = "two runs of one thing, such as a name and its words";
    }
    {
      name = "items";
      units = 4;
      between = "things on one line, such as hint and hint";
    }
    {
      name = "groups";
      units = 5;
      between = "groups on one line, and column and column";
    }
    {
      name = "inset";
      units = 6;
      between = "a frame and its text, on every side, to the ink";
    }
    {
      name = "frames";
      units = 8;
      between = "floating frames that belong together, such as the launcher and its drawer";
    }
    {
      name = "screen";
      units = 12;
      between = "a floating frame and the screen's edge";
    }
  ];

  # Each step of the type scale, as a power of its ratio: body is the face's
  # own size, and each step up multiplies it by the ratio once more. The
  # steps between are left out on purpose, so every step taken reads as one
  # (docs/design.md, Type); section is the reader's alone.
  typeSteps = {
    small = -1;
    body = 0;
    large = 1;
    section = 2;
    title = 3;
    display = 6;
    hero = 8;
  };

  power =
    x: n:
    if n == 0 then
      1.0
    else if n < 0 then
      power x (n + 1) / x
    else
      x * power x (n - 1);

  # A size to a tenth of a point, as a string, since Nix prints floats with
  # six places.
  tenths =
    x:
    let
      n = builtins.floor (x * 10 + 0.5);
    in
    "${toString (n / 10)}.${toString (n - n / 10 * 10)}";

  sizes =
    face:
    lib.mapAttrs (
      _: step:
      tenths (builtins.fromJSON cfg.font.${face}.size * power (builtins.fromJSON cfg.type.ratio) step)
    ) cfg.type.steps;

  # The serif's sizes, the mono's grown by their capitals' ratio, to a
  # hundredth of a point, since a tenth would set its capitals a fraction of
  # a pixel off the mono's.
  serifSizes = lib.mapAttrs (
    _: step:
    let
      n = builtins.floor (
        builtins.fromJSON cfg.font.mono.size
        * builtins.fromJSON cfg.font.serif.capitals
        * power (builtins.fromJSON cfg.type.ratio) step
        * 100
        + 0.5
      );
      cents = n - n / 100 * 100;
    in
    "${toString (n / 100)}.${lib.fixedWidthNumber 2 cents}"
  ) cfg.type.steps;

  # The marks every surface draws from, looked up by meaning, each with a
  # stand-in a console font can draw (glyphs.nix).
  marks = import ./glyphs.nix { inherit lib; };

  # A level of the type hierarchy: how one kind of run of text is set,
  # wherever a surface sets it (docs/design.md, Type).
  level = lib.types.submodule {
    options = {
      face = lib.mkOption {
        type = lib.types.enum [
          "mono"
          "serif"
        ];
        description = ''
          The face it's set in: the mono for a thing's name and a figure, the
          serif for words to read and the voice.
        '';
      };
      step = lib.mkOption {
        type = lib.types.str;
        description = "The type scale's step it's set at, by name.";
      };
      pitch = lib.mkOption {
        type = lib.types.ints.positive;
        description = ''
          Its line pitch in logical pixels, baseline to baseline: how far
          apart two lines of one thing set at it sit (docs/design.md, Space).
        '';
      };
      wallpaper = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = ''
          The larger step it's set at standing alone on the wallpaper, as the
          lock's clock does, rather than on a plate; none for a level that's
          only ever on one.
        '';
      };
      weight = lib.mkOption {
        type = lib.types.ints.between 100 900;
        default = 400;
        description = ''
          The mono's weight, one of the weights it's cut at. The serif, a
          variable font, is weighted by its `wght` axis instead.
        '';
      };
      axes = lib.mkOption {
        type = lib.types.attrsOf lib.types.number;
        default = { };
        description = "The serif's variable axes, its weight among them, by their tags.";
      };
      italic = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether it's set in the italic, which means the voice and nothing else.";
      };
      strong = lib.mkOption {
        type = lib.types.nullOr (lib.types.ints.between 100 900);
        default = null;
        description = ''
          The weight of a strong span in a line at this level, such as a name
          in a sentence; such a line takes its weight from the face rather
          than its axis, since Qt's rich text can't set an axis in a span.
        '';
      };
      case = lib.mkOption {
        type = lib.types.enum [
          "as-written"
          "capitals"
        ];
        default = "as-written";
        description = "Whether it's set as written or in capitals, which say label.";
      };
      tracking = lib.mkOption {
        type = lib.types.number;
        default = 0;
        description = "Its letter spacing in ems, for a level spaced other than as drawn.";
      };
      features = lib.mkOption {
        type = lib.types.listOf (lib.types.strMatching "[a-z0-9]{4}");
        default = [ ];
        description = "The OpenType features it's drawn with, by their tags.";
      };
      colour = lib.mkOption {
        type = lib.types.enum (lib.attrNames colours);
        description = ''
          The colour role it's drawn in where nothing says otherwise: beside
          a name, on the selection or in a state, a surface names another.
        '';
      };
    };
  };

  keyFace = lib.types.submodule {
    options = {
      text = lib.mkOption {
        type = lib.types.str;
        description = "What the key's cap shows.";
      };
      wide = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Whether it's an icon mark, drawn wider than a letter and given room.";
      };
    };
  };

  glyph = lib.types.submodule {
    options = {
      glyph = lib.mkOption {
        type = lib.types.str;
        description = "The mark, in the monospace face.";
      };
      console = lib.mkOption {
        type = lib.types.str;
        description = "The mark a console font can draw, for a surface on the console.";
      };
      means = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "What the mark means, wherever it's drawn.";
      };
      drawn = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = ''
          The mark drawn, for where it stands too small for a font's glyph to
          read, as the margin's corner: an SVG path on a 12 by 12 grid,
          filled even-odd, touching the grid's top and bottom and centred
          across it. Empty draws the glyph instead.
        '';
      };
    };
  };
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

    title = lib.mkOption {
      type = lib.types.str;
      default = theme.name or cfg.name;
      defaultText = lib.literalMD "the theme file's name";
      description = ''
        The theme's name as people read it, such as Osaka Jade: what a
        program that lists themes or palettes calls it.
      '';
    };

    appearance = lib.mkOption {
      type = lib.types.enum [
        "dark"
        "light"
      ];
      default = theme.appearance;
      defaultText = lib.literalMD "the theme file's";
      description = ''
        Whether the theme is light text on dark or dark on light, which
        programs and websites are told through the desktop's colour scheme.
      '';
    };

    icons = lib.mkOption {
      type = lib.types.str;
      default = theme.icons or "Yaru-sage";
      defaultText = lib.literalMD "the theme file's, or Yaru's sage";
      description = "The icon theme programs draw their icons from, one of Yaru's.";
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
        features = lib.mkOption {
          type = lib.types.listOf (lib.types.strMatching "[a-z0-9]{4}");
          # Monaspace's coding ligatures, each a stylistic set off by default;
          # texture healing is its contextual alternates, on by default. Each
          # set kept still reads as the operator it stands for: equals (!=
          # as an unequal sign), comparisons, arrows, tags, Markdown's marks,
          # the centred colon, the double arrows and the other tags. Two are
          # left off: the F# shapes draw |> and <| as triangles, which are
          # characters of their own, and the centred period draws a range
          # such as 1..=10 in middle dots.
          default = [
            "ss01"
            "ss02"
            "ss03"
            "ss04"
            "ss06"
            "ss07"
            "ss09"
            "ss10"
          ];
          description = ''
            The OpenType features the monospaced face is drawn with wherever
            fontconfig is asked for it, by their four-letter tags.
          '';
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
      serif = {
        family = lib.mkOption {
          type = lib.types.str;
          # Joel, 2 November: "i love serif, i'm willing to give fraunces a
          # try".
          default = "Fraunces";
          description = ''
            The serif face, for the desktop's voice: what the launcher says
            to him rather than what he typed or picked.
          '';
        };
        italic = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Whether the serif face is set in its italic.";
        };
        weight = lib.mkOption {
          type = lib.types.ints.between 100 900;
          # A touch under regular, as the page's text is, so a light italic
          # on a dark ground doesn't swell.
          default = 380;
          description = "The serif face's weight, on its weight axis.";
        };
        axes = lib.mkOption {
          type = lib.types.attrsOf lib.types.number;
          # Softened all the way, its wonky letters on, and drawn for the
          # size a heading reads at rather than a poster's.
          default = {
            SOFT = 100;
            WONK = 1;
            opsz = 24;
          };
          description = ''
            The serif face's variable axes but its weight, by their tags, as
            the variable font takes them.
          '';
        };
        upright = lib.mkOption {
          type = lib.types.attrsOf (lib.types.attrsOf lib.types.number);
          # Look 3's plate: a thing's name at section, a touch heavier than
          # the voice and drawn for its size, and the time at display, regular,
          # drawn for a poster's size with its wonky letters off, so its
          # figures stand plain. At large, the words the shell sets for
          # reading, as the reader's page sets its text: the voice's weight,
          # half softened, its wonky letters off and drawn for text's size,
          # since Joel is "not a fan of sans-serif" (4 November).
          default = {
            large = {
              wght = 380;
              SOFT = 50;
              WONK = 0;
              opsz = 14;
            };
            section = {
              wght = 500;
              opsz = 72;
            };
            display = {
              wght = 400;
              WONK = 0;
              opsz = 144;
            };
          };
          description = ''
            The serif face set upright, by the type scale's step it's set at:
            the axes, weight among them, laid over `font.serif.axes`.
          '';
        };
        slanted = lib.mkOption {
          type = lib.types.attrsOf (lib.types.attrsOf lib.types.number);
          # The seconds after Now's time, in the voice's italic at section,
          # regular and drawn for their size beside the time's.
          default = {
            section = {
              wght = 400;
              opsz = 72;
            };
          };
          description = ''
            The serif face's italic at a step larger than the voice's own, by
            the step: the axes, weight among them, laid over
            `font.serif.axes`.
          '';
        };
        text = {
          family = lib.mkOption {
            type = lib.types.str;
            default = "${cfg.font.serif.family} Text";
            defaultText = lib.literalExpression ''"''${font.serif.family} Text"'';
            description = ''
              The serif face upright at large, cut at those axes into a font
              of its own (../fonts/serif-text.nix), for the words a program
              sets for reading that can't set a variable font's axes: mpv's
              subtitles and the camera's panel, the boot screen and inlyne.
              The variable font's own default is drawn for 144 points.
            '';
          };
        };
        tracking = lib.mkOption {
          type = lib.types.attrsOf lib.types.number;
          # Large figures drawn at their own spacing stand apart; a hundredth
          # of an em closer holds the time as one word.
          default = {
            display = -1.0e-2;
          };
          description = ''
            The serif face's letter spacing in ems, by the type scale's step,
            for a step it's spaced other than as drawn.
          '';
        };
        capitals = lib.mkOption {
          type = number;
          # The mono face's capitals stand 0.730 of its em and the serif's
          # 0.700, so the serif is drawn 0.730 / 0.700 as large.
          default = "1.042857";
          description = ''
            How much larger than the mono face the serif face is drawn at
            each step, so their capitals stand as tall: it's sized by
            capitals rather than by its em.
          '';
        };
      };
      script = {
        families = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          # scripts.nix says which and why.
          default = (import ./scripts.nix).families;
          description = ''
            The script face, one family a script, for the scripts the theme's
            own faces lack: a letter none of them has is drawn from these, in
            turn, before any other font the machine has.
          '';
        };
        serif = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = (import ./scripts.nix).serif;
          description = ''
            The script face's cut to set beside the serif face, by script
            (hebrew, arabic, chinese, japanese), for words a surface knows
            are in that script, such as a calendar's date in its own.
          '';
        };
        mono = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = (import ./scripts.nix).mono;
          description = ''
            The script face's cut to set beside the mono face, by script, as
            `font.script.serif` is beside the serif.
          '';
        };
      };
    };

    space = {
      unit = lib.mkOption {
        type = lib.types.ints.positive;
        default = given "space" "unit" 4;
        description = ''
          The unit every space is a whole number of, in logical pixels. At a
          display scale of 2 any whole number lands on device pixels.
        '';
      };
    }
    // lib.mapAttrs (
      step: units:
      lib.mkOption {
        type = lib.types.ints.unsigned;
        default = given "space" step (cfg.space.unit * units);
        defaultText = lib.literalMD "${toString units} × `space.unit`";
        description = ''
          The ${step} step of the space scale, in logical pixels. Padding,
          margins and gaps name a step, never a number.
        '';
      }
    ) spaceSteps
    // {
      beat = lib.mkOption {
        type = lib.types.ints.positive;
        default = given "space" "beat" cfg.space.m;
        defaultText = lib.literalMD "`space.m`";
        description = ''
          The beat, in logical pixels: half the large pitch, and what every
          space between blocks down a surface is a whole number of, so lines
          at different pitches side by side meet on one grid.
        '';
      };
      rung = lib.listToAttrs (
        map (
          rung:
          lib.nameValuePair rung.name (
            lib.mkOption {
              type = lib.types.ints.positive;
              default = cfg.space.unit * rung.units;
              defaultText = lib.literalMD "${toString rung.units} × `space.unit`";
              description = ''
                The ${rung.name} rung of the space ladder, in logical pixels:
                the space between ${rung.between}. A surface names a rung,
                never a step.
              '';
            }
          )
        ) spaceRungs
      );
    };

    type = {
      ratio = lib.mkOption {
        type = number;
        # A major third: steps far enough apart to read as different, near
        # enough that a heading doesn't shout.
        default = given "type" "ratio" "1.25";
        description = "How much bigger each step of the type scale is than the one below.";
      };
      steps = lib.mkOption {
        type = lib.types.attrsOf lib.types.int;
        default = typeSteps // given "type" "steps" { };
        defaultText = lib.literalExpression (lib.generators.toPretty { } typeSteps);
        description = ''
          The steps of the type scale, each as how many times the ratio
          multiplies the face's size: body is 0, a step smaller is -1.
        '';
      };
      mono = lib.mkOption {
        type = lib.types.attrsOf number;
        default = sizes "mono";
        defaultText = lib.literalMD "`font.mono.size` times the ratio to each step's power";
        description = "Each step's size in points, in the monospaced face.";
      };
      sans = lib.mkOption {
        type = lib.types.attrsOf number;
        default = sizes "sans";
        defaultText = lib.literalMD "`font.sans.size` times the ratio to each step's power";
        description = "Each step's size in points, in the proportional face.";
      };
      eyebrow = {
        tracking = lib.mkOption {
          type = lib.types.number;
          # Small capitals set apart, so a line above a name reads as a
          # label rather than a word. One tracking serves every eyebrow,
          # the launcher's plate's and the reader's, so the same kind of
          # line reads the same wherever it heads something.
          default = 0.16;
          description = ''
            How far apart an eyebrow's capitals are set, in ems: the line
            above a name saying when, where or who.
          '';
        };
      };
      serif = lib.mkOption {
        type = lib.types.attrsOf number;
        default = serifSizes;
        defaultText = lib.literalMD "`type.mono`'s sizes times `font.serif.capitals`";
        description = "Each step's size in points, in the serif face, its capitals as tall as the mono's.";
      };
      levels = lib.mkOption {
        type = lib.types.attrsOf level;
        # The hierarchy of 4 November (docs/design.md, Type): each level
        # differs from its neighbours in at least two of size, weight, face,
        # case and colour, since one step of one alone reads as texture
        # rather than rank. Words and the voice are the serif as the shell
        # set it before there were levels; label is note's variant, told
        # apart by its case.
        default = {
          display = {
            face = "mono";
            step = "display";
            # Figures alone, which never descend, so they sit close.
            pitch = 56;
            wallpaper = "hero";
            tracking = -1.0e-2;
            features = [ "tnum" ];
            colour = "text";
          };
          # Drawn for the size it's set at, never a poster's cut, and with
          # the threshold's whimsy: softened, its wonky letters on.
          # A surface's head, at its line's size: it stands apart from the
          # glyph and the text beside it by a heavy serif in strong ink,
          # never by size (Joel, 2026-11-06: "use weights and not size for
          # that ... it feels like shit"). Heavier than Words' strong, which
          # stays for emphasis inside words, and no tighter, since that was
          # for a display cut.
          title = {
            face = "serif";
            step = "large";
            pitch = 24;
            axes = {
              wght = 640;
              opsz = 14;
              SOFT = 100;
              WONK = 1;
            };
            colour = "strong";
          };
          # Medium always, so selection changes only colour and fill and a
          # weight means only a level; the mono swells at bold on a dark
          # ground.
          name = {
            face = "mono";
            step = "large";
            pitch = 24;
            weight = 500;
            colour = "text";
          };
          figure = {
            face = "mono";
            step = "large";
            pitch = 24;
            features = [ "tnum" ];
            colour = "text";
          };
          words = {
            face = "serif";
            step = "large";
            pitch = 24;
            axes = cfg.font.serif.axes // cfg.font.serif.upright.large;
            strong = 600;
            colour = "muted";
          };
          voice = {
            face = "serif";
            step = "large";
            pitch = 24;
            italic = cfg.font.serif.italic;
            axes = cfg.font.serif.axes // {
              wght = cfg.font.serif.weight;
            };
            colour = "muted";
          };
          note = {
            face = "mono";
            step = "body";
            pitch = 20;
            colour = "muted";
          };
          label = {
            face = "mono";
            step = "body";
            pitch = 20;
            weight = 500;
            case = "capitals";
            tracking = cfg.type.eyebrow.tracking;
            features = [ "case" ];
            colour = "muted";
          };
        };
        defaultText = lib.literalMD "display, title, name, figure, words, voice, note and label";
        description = ''
          The type hierarchy's levels, by name: how each kind of run of text
          is set, so a surface names a level rather than a size or a weight.
        '';
      };
    };

    motion = {
      spring = {
        damping = lib.mkOption {
          type = number;
          # Critically damped: as fast as a spring can settle without
          # overshooting, so nothing wobbles.
          default = given "motion" "damping" "1.0";
          description = "The spring's damping ratio: 1 settles without overshooting.";
        };
        stiffness = lib.mkOption {
          type = lib.types.ints.positive;
          # Covers 98% of the distance in about 150 ms.
          default = given "motion" "stiffness" 1600;
          description = "How stiff the spring is: stiffer is quicker.";
        };
        epsilon = lib.mkOption {
          type = number;
          default = given "motion" "epsilon" "0.001";
          description = "How close to still counts as stopped.";
        };
      };
      curve = lib.mkOption {
        type = lib.types.addCheck (lib.types.listOf number) (points: lib.length points == 4);
        # A fast start that eases out, so a thing is mostly there at once.
        default = given "motion" "curve" [
          "0.23"
          "1"
          "0.32"
          "1"
        ];
        description = "The cubic Bézier, x1 y1 x2 y2, for movements with a set duration.";
      };
      short = lib.mkOption {
        type = lib.types.ints.positive;
        default = given "motion" "short" 100;
        description = "Milliseconds for something leaving, or small: a close, a fade.";
      };
      medium = lib.mkOption {
        type = lib.types.ints.positive;
        default = given "motion" "medium" 150;
        description = "Milliseconds for something arriving: a window opening, a menu.";
      };
      long = lib.mkOption {
        type = lib.types.ints.positive;
        default = given "motion" "long" 250;
        description = "Milliseconds for something large and rare: a wallpaper, the lock screen.";
      };
      slowdown = lib.mkOption {
        type = number;
        default = given "motion" "slowdown" "1";
        description = "Slows every animation by this factor, to look at one closely.";
      };
    };

    marks = lib.mkOption {
      type = lib.types.attrsOf glyph;
      default = lib.recursiveUpdate marks (theme.marks or { });
      defaultText = lib.literalMD "pi's marks, with any the theme file changes";
      description = ''
        The marks every surface draws, by what they mean, each with a stand-in
        a console font can draw. A surface looks a mark up here and never
        types one of its own.
      '';
    };

    keys = lib.mkOption {
      type = lib.types.attrsOf keyFace;
      default = import ./keys.nix { mark = name: cfg.marks.${name}.glyph; };
      defaultText = lib.literalMD "the faces in keys.nix, wearing the theme's marks";
      description = ''
        How a key is drawn wherever a surface names one, by its xkb name: its
        own glyph where a proper one exists, its name where none does. A
        surface that draws a key looks it up here; a key not here is drawn
        as its name, and a letter as itself.
      '';
    };

    shape = {
      border = lib.mkOption {
        type = lib.types.ints.unsigned;
        # Picked in step 1's studio: a hairline that frames without weight.
        default = 1;
        description = "How wide, in pixels, the border around a window or a prompt is.";
      };
      radius = lib.mkOption {
        type = lib.types.ints.unsigned;
        # One unit: softened, never round. At two units Joel found "often too
        # much roundness" (2026-11-02), and a part's curve inside it wobbled.
        default = cfg.space.xs;
        defaultText = lib.literalMD "`space.xs`";
        description = "How round, in pixels, the corners of a window or a prompt are.";
      };
      gap = lib.mkOption {
        type = lib.types.ints.unsigned;
        default = cfg.space.m;
        defaultText = lib.literalMD "`space.m`";
        description = "The space, in pixels, between windows.";
      };
      inset = lib.mkOption {
        type = lib.types.ints.unsigned;
        default = cfg.space.xs;
        defaultText = lib.literalMD "`space.xs`";
        description = ''
          How far, in pixels, a part sits inside the border of the whole it
          belongs to, such as the launcher's selected row. A part's corners
          are the radius less the border and this, never below square: at the
          default radius, square.
        '';
      };
    };

    floating = {
      opacity = lib.mkOption {
        type = number;
        # A cloud that hints at what's beneath, solid enough that muted words
        # keep Lc 60 over the brightest patch of the wallpaper, blurred: at
        # 0.85 they fell to 59.6 there, and at 0.9 they hold 60.6.
        default = given "floating" "opacity" "0.9";
        description = ''
          How opaque a surface floating above the windows is, a launcher, a
          prompt, a notice or a meter, drawn in the deepest colour over a blur
          of what's beneath.
        '';
      };
      scrim = lib.mkOption {
        type = number;
        default = given "floating" "scrim" "0.6";
        description = ''
          How opaque the deepest colour is over everything behind a prompt,
          which stops the rest of the screen while it asks.
        '';
      };
      selection = lib.mkOption {
        type = number;
        default = given "floating" "selection" "0.6";
        description = "How opaque the selection is behind a selected row on a floating surface.";
      };
      shadows = lib.mkOption {
        type = lib.types.listOf (
          lib.types.submodule {
            options = {
              offset = lib.mkOption {
                type = lib.types.ints.unsigned;
                description = "How far below the surface, in pixels, the shadow falls.";
              };
              blur = lib.mkOption {
                type = lib.types.ints.unsigned;
                description = "How soft the shadow is, in pixels.";
              };
              opacity = lib.mkOption {
                type = number;
                description = "How dark the shadow is, from 0 to 1.";
              };
            };
          }
        );
        # A close, crisp one that sets the edge and a far, soft one that
        # lifts the surface, both light.
        default = given "floating" "shadows" [
          {
            offset = 1;
            blur = 2;
            opacity = "0.2";
          }
          {
            offset = 8;
            blur = 24;
            opacity = "0.25";
          }
        ];
        description = "The shadows under a floating surface, nearest first.";
      };
    };

    wallpaper =
      let
        picture =
          key: where:
          lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = given "wallpaper" key null;
            description = ''
              The picture behind ${where}: its path in the wallpapers collection
              (github:Jitsusama/wallpapers.nix), such as
              landscapes/glowing-city-green.png, or an absolute path to any
              picture. Null draws the deepest colour instead.
            '';
          };
      in
      {
        desktop = picture "desktop" "every workspace, standing still while they move";
        lock = picture "lock" "the lock screen's time and field";
        shadows = lib.mkOption {
          type = lib.types.listOf (
            lib.types.submodule {
              options = {
                x = lib.mkOption {
                  type = lib.types.int;
                  description = "How far right of the text, in pixels, the shadow falls.";
                };
                y = lib.mkOption {
                  type = lib.types.int;
                  description = "How far below the text, in pixels, the shadow falls.";
                };
                blur = lib.mkOption {
                  type = lib.types.ints.unsigned;
                  description = "How soft the shadow is, in pixels.";
                };
                opacity = lib.mkOption {
                  type = number;
                  description = "How dark the shadow is, from 0 to 1.";
                };
              };
            }
          );
          # A crisp one set off down and right, which gives each letter an
          # edge, and a soft one around it, which keeps a bright patch of the
          # picture from reaching the letters.
          default = given "wallpaper" "shadows" [
            {
              x = 1;
              y = 2;
              blur = 2;
              opacity = "0.85";
            }
            {
              x = 0;
              y = 1;
              blur = 16;
              opacity = "0.6";
            }
          ];
          description = ''
            The shadows, in the shadow colour, that lift text set straight on
            a picture, the lock screen's time, so it reads while the picture
            shows whole. Nearest first.
          '';
        };
        halo = lib.mkOption {
          type = lib.types.listOf (
            lib.types.submodule {
              options = {
                blur = lib.mkOption {
                  type = lib.types.ints.unsigned;
                  description = "How far the shadow spreads round the letters, in pixels.";
                };
                opacity = lib.mkOption {
                  type = number;
                  description = "How dark the shadow is, from 0 to 1.";
                };
              };
            }
          );
          # Stacked from tight to wide, each nearly opaque, so the shade is
          # all but solid at the ink and feathers out to nothing, with no
          # edge of its own.
          default = given "wallpaper" "halo" (
            map
              (blur: {
                inherit blur;
                opacity = "0.95";
              })
              [
                1
                2
                3
                4
                5
                6
                8
                10
                13
                16
                24
                40
              ]
          );
          description = ''
            The shadows, in the shadow colour and centred on the letters, that
            hold text off whatever is behind it when that may be a picture or
            a window: the overview's status, which lies over the workspaces.
            Tightest first.
          '';
        };
        marks = lib.mkOption {
          type = lib.types.listOf (
            lib.types.submodule {
              options = {
                blur = lib.mkOption {
                  type = lib.types.ints.unsigned;
                  description = "How far the shadow spreads round the mark, in pixels.";
                };
                opacity = lib.mkOption {
                  type = number;
                  description = "How dark the shadow is, from 0 to 1.";
                };
              };
            }
          );
          # One tight shade, a pixel round each mark, so it holds the mark
          # off a bright patch of the picture and stays inside the room its
          # cell leaves, never reaching the window below.
          default = given "wallpaper" "marks" [
            {
              blur = 1;
              opacity = "0.8";
            }
          ];
          description = ''
            The shadows, in the shadow colour and centred on each mark, that
            lift the margin's marks off the picture. Centred, so a mark has
            the same shade on every side, and tight, so the shade stays in
            the mark's cell. Nearest first.
          '';
        };
      };
  };

  # The whole theme, for a program that reads JSON rather than having a file
  # of its own written: pi, a script, the stick's guide.
  config.xdg.configFile."jitsusama/theme.json".text = builtins.toJSON {
    inherit (cfg)
      colours
      terminal
      syntax
      shades
      font
      space
      motion
      shape
      floating
      wallpaper
      appearance
      icons
      ;
    inherit (cfg.type) mono sans;
    # Each level with its size in points worked out, as the shell's Theme
    # gives it, for a program that sets its own text, such as the reader.
    levels = lib.mapAttrs (
      _: level:
      let
        size = step: builtins.fromJSON cfg.type.${level.face}.${step};
      in
      level
      // {
        inherit (cfg.font.${level.face}) family;
        size = size level.step;
        wallpaper = if level.wallpaper == null then null else size level.wallpaper;
      }
    ) cfg.type.levels;
    name = cfg.title;
    marks = lib.mapAttrs (_: mark: { inherit (mark) glyph console; }) cfg.marks;
  };

  # Joel's aesthetic, for every agent on the machine, so one designing
  # anything in any repository builds to it rather than to its own taste,
  # with the design document beside it for the system built from it.
  config.home.file = lib.listToAttrs (
    lib.concatMap
      (skills: [
        (lib.nameValuePair "${skills}/aesthetic/SKILL.md" { source = ./SKILL.md; })
        (lib.nameValuePair "${skills}/aesthetic/design.md" { source = ../../docs/design.md; })
      ])
      [
        ".pi/agent/skills"
        ".claude/skills"
      ]
  );

  config.assertions = [
    {
      assertion = widening == [ ];
      message = ''
        The theme's space ladder has to widen outward, each rung wider than the
        one inside it (docs/design.md, Space):
        ${lib.concatMapStringsSep "\n" (
          pair:
          "  ${pair.inner} ${toString cfg.space.rung.${pair.inner}} is not under ${pair.outer} ${
              toString cfg.space.rung.${pair.outer}
            }"
        ) widening}
      '';
    }
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
