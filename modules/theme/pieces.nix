# The shape every surface's content takes (docs/design.md, Customising): a
# set of named pieces, each with its place, its tier, when it gives way to
# narrower space, the mark it wears while it's true, the file that draws it
# and settings of its own. Naming them, rather than listing them, lets a
# machine change one piece in one line or add its own beside the rest.
{ lib }:
let
  tiers = {
    always = "always shown: what's needed to act right now";
    when = "shown only while it's true: state that changes what Joel would do";
    request = "one step away, behind the thing it belongs to";
  };

  piece =
    { glyphs, sources }:
    lib.types.submodule {
      # A piece's own settings, its format or its thresholds, sit beside
      # these and go to the file that draws it as they are.
      freeformType = lib.types.attrsOf lib.types.anything;
      options = {
        order = lib.mkOption {
          type = lib.types.int;
          description = "Where it sits, lowest first.";
        };
        tier = lib.mkOption {
          type = lib.types.nullOr (lib.types.enum (lib.attrNames tiers));
          default = "always";
          description = ''
            Its tier: ${lib.concatStringsSep "; " (lib.mapAttrsToList (name: what: "${name}, ${what}") tiers)}.
            Null leaves it out.
          '';
        };
        collapse = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = null;
          description = ''
            When it gives way as space runs short: the lowest rank goes
            first. Null keeps it whatever the space.
          '';
        };
        mark = lib.mkOption {
          type = lib.types.nullOr (lib.types.enum (lib.attrNames glyphs));
          default = null;
          description = "The theme's glyph it wears while it's true, by name.";
        };
        source = lib.mkOption {
          type = lib.types.either (lib.types.enum (lib.attrNames sources)) lib.types.path;
          description = "What draws it: a built-in piece by name, or a file of the same shape.";
        };
      };
    };

  # The pieces a surface draws, in order, each with its name, and without
  # the ones left out.
  drawn =
    pieces:
    lib.sortOn (piece: piece.order) (
      lib.mapAttrsToList (name: piece: piece // { inherit name; }) (
        lib.filterAttrs (_: piece: piece.tier != null) pieces
      )
    );
in
{
  inherit tiers piece drawn;
}
