# The theme's space: the unit, the scale's steps and the ladder of rungs
# that sets levels of structure apart, held to widen outward as the machine
# is evaluated (docs/design.md, Space).
{
  config,
  lib,
  ...
}:
let
  cfg = config.jitsusama.theme;
  inherit (import ./shared.nix { inherit lib cfg; }) given;

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

  # Each pair of neighbouring rungs whose outer isn't wider than its inner,
  # which would let two levels of structure share a space.
  widening = lib.filter (pair: cfg.space.rung.${pair.inner} >= cfg.space.rung.${pair.outer}) (
    lib.zipListsWith (inner: outer: {
      inner = inner.name;
      outer = outer.name;
    }) spaceRungs (lib.tail spaceRungs)
  );
in
{
  options.jitsusama.theme = {
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
  };

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
  ];
}
