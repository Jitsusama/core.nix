# The look every program Joel sees shares, read from one theme file beside
# this module (docs/design.md says what a theme holds and why). A program's
# module reads it from jitsusama.theme and writes it in the program's own
# format, so a terminal, a notification and the lock screen all look alike,
# and picking another theme changes them all. Surfaces name what a colour
# means (its colours), never the colour itself, and the theme is checked for
# legibility as the machine is evaluated.
#
# Its colours, type and space are modules of their own beside it, by the
# design's sections (colour.nix, type.nix, space.nix); this one holds the rest
# and writes the theme out for programs that read it whole.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.theme;
  inherit (import ./shared.nix { inherit lib cfg; }) theme given number;

  themes = {
    osaka-jade = ./osaka-jade.toml;
    flexoki-light = ./flexoki-light.toml;
  };

  # The marks every surface draws from, looked up by meaning, each with a
  # stand-in a console font can draw (glyphs.nix).
  marks = import ./glyphs.nix { inherit lib; };

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
  imports = [
    ./colour.nix
    ./space.nix
    ./type.nix
  ];

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

  # The theme for a website, which a site copies whole rather than retyping
  # the theme's values (web/web.nix).
  config.xdg.dataFile."jitsusama/web".source = import ./web/web.nix { inherit pkgs cfg themes; };

  # Joel's aesthetic, for every agent on the machine, so one designing
  # anything in any repository builds to it rather than to its own taste,
  # with the design document beside it for the system built from it; and
  # how a website is built to it, with its references.
  config.home.file = lib.listToAttrs (
    lib.concatMap
      (skills: [
        (lib.nameValuePair "${skills}/aesthetic/SKILL.md" { source = ./SKILL.md; })
        (lib.nameValuePair "${skills}/aesthetic/design.md" { source = ../../docs/design.md; })
        (lib.nameValuePair "${skills}/web-design" { source = ./web/skill; })
      ])
      [
        ".pi/agent/skills"
        ".claude/skills"
      ]
  );
}
