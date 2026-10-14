# jq, its JSON drawn as bat draws a file: keys in the text colour, strings
# and values in the terminal's colours bat gives them, and the brackets,
# commas and nulls between them muted.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) roles terminal;
  # jq takes a colour as the escape's own numbers.
  sgr =
    colour:
    "38;2;"
    + lib.concatMapStringsSep ";" (at: toString (lib.fromHexString (builtins.substring at 2 colour))) [
      1
      3
      5
    ];
in
{
  imports = [ ../theme/home.nix ];

  programs.jq = {
    enable = true;
    colors = {
      null = sgr roles.muted;
      false = sgr terminal.bright_red;
      true = sgr terminal.bright_red;
      numbers = sgr terminal.bright_red;
      strings = sgr terminal.green;
      arrays = sgr roles.muted;
      objects = sgr roles.muted;
      objectKeys = sgr roles.text;
    };
  };
}
