# The checks of the launcher's emoji, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
{
  lib,
  noFonts,
  pkgs,
  timedQml,
  ...
}:
{
  # The launcher finds emoji by what they're called and what they mean, best
  # first, as tests/emoji.qml lists them, over the list it's built with, and
  # in about three times the least a search could take.
  the-launcher-finds-emoji =
    let
      emoji = import ../launcher/emoji.nix { inherit lib pkgs; } "none";
    in
    pkgs.runCommandLocal "the-launcher-finds-emoji" { } ''
      mkdir launcher
      cp ${../../../tests/emoji.qml} emoji.qml
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Emoji.js} launcher/Emoji.js
      cp ${emoji} emoji.jsonl
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export QML_XHR_ALLOW_FILE_READ=1
      export FONTCONFIG_FILE=${noFonts}
      ${timedQml "emoji.qml"}
      touch $out
    '';
}
