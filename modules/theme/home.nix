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
    voice = "a glyph for the shell speaking for itself: an empty list, a greeting";
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

  # Each step of the type scale, as a power of its ratio: body is the face's
  # own size, and each step up multiplies it by the ratio once more.
  typeSteps = {
    small = -1;
    body = 0;
    large = 1;
    title = 2;
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

  # The marks every surface draws from, looked up by meaning, each with a
  # stand-in a console font can draw (glyphs.nix).
  glyphs = import ./glyphs.nix { inherit lib; };

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

    roles = lib.mapAttrs (
      name: description: colorOption theme.roles.${name} "The colour for ${description}."
    ) roles;

    terminal = lib.mapAttrs (
      name: description: colorOption theme.terminal.${name} "The colour for ${description}."
    ) terminal;

    syntax = lib.mapAttrs (name: about: {
      colour = lib.mkOption {
        type = color;
        default =
          if written name ? pigment then pigment (written name).pigment else cfg.roles.${about.role};
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
    ) spaceSteps;

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

    glyphs = lib.mkOption {
      type = lib.types.attrsOf glyph;
      default = lib.recursiveUpdate glyphs (theme.glyphs or { });
      defaultText = lib.literalMD "pi's marks, with any the theme file changes";
      description = ''
        The marks every surface draws, by what they mean, each with a stand-in
        a console font can draw. A surface looks a mark up here and never
        types one of its own.
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
        default = cfg.space.s;
        defaultText = lib.literalMD "`space.s`";
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
          are the radius less the border and this, so the curves run parallel.
        '';
      };
    };

    floating = {
      opacity = lib.mkOption {
        type = number;
        # Picked in step 1's studio: a cloud that hints at what's beneath.
        default = given "floating" "opacity" "0.85";
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
            a picture, the lock screen's time and the margin's pulls, so it
            reads while the picture shows whole. Nearest first.
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
      };
  };

  # The whole theme, for a program that reads JSON rather than having a file
  # of its own written: pi, a script, the stick's guide.
  config.xdg.configFile."jitsusama/theme.json".text = builtins.toJSON {
    inherit (cfg)
      roles
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
    name = cfg.title;
    glyphs = lib.mapAttrs (_: mark: { inherit (mark) glyph console; }) cfg.glyphs;
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
