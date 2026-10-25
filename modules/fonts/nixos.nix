# The theme's fonts, and fontconfig keeping GitHub's pages at their width.
#
# The theme's sans face ships seven variable files that each call themselves
# by its name and "VF" at Regular: the condensed, expanded and italic widths
# beside the one whose width and slant are axes. Asked for that name,
# fontconfig gives the condensed one first, and GitHub asks for it by name
# before any other face, so its pages drew in condensed type. Only the file with every axis is kept
# under that name; it draws each width and slant the others did. A Mac reads
# no fontconfig, so this is NixOS's alone.
{ pkgs, ... }:
let
  truetype = "${pkgs.mona-sans}/share/fonts/truetype";
in
{
  imports = [ ./system.nix ];

  fonts.fontconfig.localConf = ''
    <?xml version="1.0"?>
    <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
    <fontconfig>
      <selectfont>
        <rejectfont>
          <glob>${truetype}/MonaSansVF-*</glob>
          <glob>${truetype}/MonaSansVF[opsz,wght].ttf</glob>
          <glob>${truetype}/MonaSansVF[wdth,opsz,wght].ttf</glob>
        </rejectfont>
      </selectfont>
    </fontconfig>
  '';
}
