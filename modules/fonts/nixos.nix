# The theme's fonts, and fontconfig answering web pages as a Mac would.
#
# The theme's sans face ships seven variable files that each call themselves
# by its name and "VF" at Regular: the condensed, expanded and italic widths
# beside the one whose width and slant are axes. Asked for that name,
# fontconfig gives the condensed one first, and GitHub asks for it by name
# before any other face, so its pages drew in condensed type. Only the file
# with every axis is kept under that name; it draws each width and slant the
# others did.
#
# Pages ask for the Mac's own faces by name, its system font and its
# monospaced one, and on a Mac get San Francisco. Here they got a stand-in
# Chrome picked, so GitHub's code drew in Liberation Mono. The theme's faces
# answer to those names too, so such pages draw in the account's sans and
# monospaced faces, as they draw in the Mac's. Chrome takes a face only under
# a name it carries, not one fontconfig would substitute, so the face found
# carries those names too: added as it's matched rather than as it's read,
# since NixOS's font cache is made without this file. A Mac reads no
# fontconfig, so this is NixOS's alone.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  truetype = "${pkgs.mona-sans}/share/fonts/truetype";
  account = if options ? jitsusama.login.account then config.jitsusama.login.account else null;
  home =
    if account != null && options ? home-manager then
      config.home-manager.users.${account} or null
    else
      null;
  font = lib.attrByPath [ "jitsusama" "theme" "font" ] null home;
  # The faces a page names for a Mac's system font and its monospaced one.
  macNames = {
    sans = [
      "-apple-system"
      "BlinkMacSystemFont"
      "ui-sans-serif"
    ];
    mono = [
      "ui-monospace"
      "SFMono-Regular"
      "SF Mono"
      "Menlo"
    ];
  };
  answers =
    family: names:
    lib.optionalString (font != null) (
      lib.concatMapStrings (name: ''
        <alias binding="same">
          <family>${name}</family>
          <prefer><family>${family}</family></prefer>
        </alias>
      '') names
      + ''
        <match target="font">
          <test name="family"><string>${family}</string></test>
          <edit name="family" mode="append_last">
        ${lib.concatMapStrings (name: "      <string>${name}</string>\n") names}    </edit>
        </match>
      ''
    );
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
    ${answers font.sans.family macNames.sans}${answers font.mono.family macNames.mono}</fontconfig>
  '';
}
