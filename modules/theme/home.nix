# The look Joel's programs share: Omarchy's Osaka Jade colours, which a
# program's module reads from jitsusama.theme.colors and writes in its own
# format. Each colour is an option of its own, so a machine can change one and
# keep the rest.
{ lib, ... }:
let
  # Omarchy reads `mode` to choose a light or dark GTK theme; it isn't a
  # colour, and nothing here reads it yet.
  osakaJade = removeAttrs (builtins.fromTOML (builtins.readFile ./osaka-jade.toml)) [ "mode" ];
in
{
  options.jitsusama.theme.colors = lib.mapAttrs (
    name: color:
    lib.mkOption {
      type = lib.types.strMatching "#[0-9A-Fa-f]{6}";
      default = color;
      description = "The theme's ${name} colour, as #RRGGBB.";
    }
  ) osakaJade;
}
