# The look every program Joel sees shares: Omarchy's Osaka Jade colours, one
# monospaced font, and one shape (the border, corners and gaps). A program's
# module reads them from jitsusama.theme and writes them in the program's own
# format, so a terminal, a prompt, a notification and the lock screen all
# look alike. Each is an option of its own, so a machine can change one and
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

  options.jitsusama.theme.font = {
    family = lib.mkOption {
      type = lib.types.str;
      default = "MonaspiceXe Nerd Font";
      description = "The font every program draws its text in.";
    };
    size = lib.mkOption {
      # A string, since Nix prints the number 10.5 as 10.500000.
      type = lib.types.strMatching "[0-9]+(\\.[0-9]+)?";
      # WezTerm's 14 on macOS: macOS counts 72 points per inch and Linux 96,
      # so the same size is a smaller number here.
      default = "10.5";
      description = "The font's size in points, as Linux counts them.";
    };
  };

  options.jitsusama.theme.shape = {
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
}
