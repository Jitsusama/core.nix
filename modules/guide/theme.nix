# The theme as the guide draws it on a terminal, rendered into its build:
# each role it uses as the SGR number of the terminal colour holding it, as
# the console module reads an account's theme (../console/nixos.nix), and
# each of the theme's marks with its console stand-in (../theme/glyphs.nix).
# The stick has no account, so the guide names a theme file of its own, and
# the stick's console takes its sixteen colours from the same file.
{
  lib,
  theme ? ../theme/osaka-jade.toml,
}:
let
  read = builtins.fromTOML (builtins.readFile theme);

  eight = [
    "black"
    "red"
    "green"
    "yellow"
    "blue"
    "magenta"
    "cyan"
    "white"
  ];
  names = eight ++ map (name: "bright_${name}") eight;

  # The SGR number of the terminal colour holding the role's pigment, or
  # the terminal's own foreground when none does.
  slot =
    role:
    let
      pigment = read.roles.${role} or null;
      at = lib.lists.findFirstIndex (name: (read.terminal.${name} or null) == pigment) null names;
    in
    if pigment == null || at == null then "39" else toString (if at < 8 then 30 + at else 82 + at);

  # The roles the guide draws in: text in its three, the accent where the
  # keys go, the states, and the hues its marks wear.
  roles = [
    "text"
    "strong"
    "muted"
    "accent"
    "alert"
    "warning"
    "success"
    "action"
    "machine"
    "voice"
  ];
in
{
  # What the guide's build reads with option_env!.
  env = {
    GUIDE_ROLES = lib.concatMapStringsSep " " (role: "${role}=${slot role}") roles;
    GUIDE_GLYPHS = lib.concatStringsSep "\n" (
      lib.mapAttrsToList (name: mark: "${name} ${mark.glyph} ${mark.console}") (
        import ../theme/glyphs.nix { inherit lib; }
      )
    );
  };
  # The console's sixteen, in its order, as console.colors takes them.
  colours = map (name: lib.removePrefix "#" read.palette.${read.terminal.${name}}) names;
}
