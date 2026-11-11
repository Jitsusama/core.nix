# The theme's type: the faces, the scale their sizes step by and the levels
# of the hierarchy each run of text is set at (docs/design.md, Type).
{
  config,
  lib,
  ...
}:
let
  cfg = config.jitsusama.theme;
  inherit (import ./shared.nix { inherit lib cfg; }) given number colours;

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
in
{
  options.jitsusama.theme = {
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
  };
}
