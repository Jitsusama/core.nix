# The checks of the launcher's emoji, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
{
  account,
  copied,
  graphical,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's Emoji keeps a search the clipboard's history left
  # unanswered, and one typed inside a group, only while the launcher is open,
  # so its rows follow the launcher's open (modules/quickshell/tests/emoji.qml),
  # with nothing at the history's socket and niri stood in.
  emoji-keeps-a-lost-search-only-open =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Quickshell runs them with.
    pkgs.runCommandLocal "emoji-keeps-a-lost-search-only-open" { } ''
      mkdir -p launcher runtime && chmod -R 700 runtime
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [
        "Daemon.qml"
        "Options.qml"
        "Pasting.qml"
        "Words.qml"
        "launcher/Emoji.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${singleton "Niri.qml" ''
        property var current: null
        property var windows: ({})
        property var byApp: ({})
        function spawn(command) { }
      ''} Niri.qml
      cp ${../tests/emoji.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The clipboard's history finds emoji by what they're called and what they
  # mean, best first, over the list the launcher is built with: its tests run
  # as it builds, with EMOJI_LIST naming the list, which its search test reads
  # (clipboard/src/search.rs), and EMOJI_LIST_WANTED failing that test if the
  # list never reaches it, rather than it passing having read nothing.
  the-launcher-finds-emoji = (pkgs.callPackage ../clipboard/package.nix { }).overrideAttrs {
    EMOJI_LIST = import ../launcher/emoji.nix { inherit lib pkgs; } "none";
    EMOJI_LIST_WANTED = "1";
  };
}
