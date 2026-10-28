# ripgrep, drawn as delta draws a diff: each file named in the strongest
# colour, line numbers muted, and a match marked as the launcher marks its
# chosen row, in the selection colour rather than the alert's red.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) colours;
  # ripgrep takes a colour as three hex bytes, 0xrr,0xgg,0xbb.
  bytes =
    colour:
    lib.concatMapStringsSep "," (at: "0x${builtins.substring at 2 colour}") [
      1
      3
      5
    ];
in
{
  imports = [ ../theme/home.nix ];

  programs.ripgrep = {
    enable = true;
    arguments = [
      "--max-columns-preview"
      "--colors=path:fg:${bytes colours.strong}"
      "--colors=path:style:bold"
      "--colors=line:fg:${bytes colours.muted}"
      "--colors=column:fg:${bytes colours.muted}"
      "--colors=match:fg:${bytes colours.strong}"
      "--colors=match:bg:${bytes colours.selection}"
      "--colors=match:style:nobold"
    ];
  };
}
