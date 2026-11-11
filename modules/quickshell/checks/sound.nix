# The checks of the launcher's Sound, the music playing, and dictation, which
# the folder's checks.nix gathers with every other place's; its opening comment
# says why each place keeps its own.
{
  account,
  graphical,
  lib,
  noFonts,
  onLinux,
  pkgs,
  qs,
  shellFolderLint,
  singleton,
  ...
}:
{
  # The launcher's Sound never listens to a headset's microphone while the
  # headset plays music, and makes a device's parts and drawer
  # (tests/sound.qml), over stand-in devices, with niri and the music stood
  # in for; and it lists the rows the sound daemon serves, from a stand-in
  # serving them on its socket (tests/sound-served.qml).
  sound-hears-only-what-can-be-heard =
    let
      niri = singleton "Niri.qml" ''
        property var workspaces: ({ "1": { id: 1, output: "eDP-1" } })
      '';
      music = singleton "Music.qml" ''
        property bool playing: false
        property string title: ""
        property string artist: ""
      '';
    in
    pkgs.runCommandLocal "sound-hears-only-what-can-be-heard" { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      cp ${../Fuzzy.js} Fuzzy.js
      cp ${../launcher/Provider.qml} launcher/Provider.qml
      cp ${../ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../launcher/Asking.js} launcher/Asking.js
      cp ${../launcher/Numbers.js} launcher/Numbers.js
      cp ${../launcher/Sounds.js} launcher/Sounds.js
      cp ${../launcher/Targets.js} launcher/Targets.js
      cp ${../launcher/Plate.js} launcher/Plate.js
      cp ${../launcher/Sound.qml} launcher/Sound.qml
      cp ${../launcher/Order.js} launcher/Order.js
      cp ${../launcher/ReadyRows.qml} launcher/ReadyRows.qml
      cp ${../launcher/ReadyRows.js} launcher/ReadyRows.js
      cp ${../launcher/Ago.js} launcher/Ago.js
      cp ${../launcher/Interfaces.js} launcher/Interfaces.js
      cp ${../When.js} When.js
      cp ${../Daemon.qml} Daemon.qml
      cp ${../Hearing.js} Hearing.js
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../Words.qml} Words.qml
      cp ${../launcher/words.json} launcher/words.json
      cp ${niri} Niri.qml
      cp ${music} Music.qml
      cp ${../../../tests/sound.qml} shell.qml
      cp ${../../../tests/sound-served.qml} served.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      mkdir -p runtime/daemons
      timeout 60 ${qs} -p served.qml
      touch $out
    '';

  # The music is the player that matters: of those playing, the last to
  # start, else the last that played, which a play key resumes, and never
  # playerctld's echo of another, as tests/music.qml lists them, with where
  # in the track said as the card says it.
  the-music-follows-the-player =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-music-follows-the-player" { } ''
      cp ${../../../tests/music.qml} music.qml
      cp ${../Music.js} Music.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml music.qml
      touch $out
    '';

  # A media player's drawer says all its player tells MPRIS, in the order
  # every drawer keeps, and draws its art only from where it may, as
  # tests/media.qml holds launcher/Media.js and Pictures.js to players
  # made up there.
  a-player-says-all-it-knows =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-player-says-all-it-knows" { } ''
      mkdir launcher
      cp ${../../../tests/media.qml} media.qml
      cp ${../Music.js} Music.js
      cp ${../Pictures.js} Pictures.js
      cp ${../launcher/Media.js} launcher/Media.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml media.qml
      touch $out
    '';

  # The shell follows dictation as its service says (tests/dictation-watch.py
  # stands in for it): each state said, a service that stops watched again
  # once its socket starts it, and a socket nothing listens on tried once,
  # never in a loop.
  the-desktop-follows-dictation =
    let
      niri = pkgs.writeText "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var windows: ({})
          property var casts: ({})
          function spawn(command) { }
          function act(request) { }
        }
      '';
    in
    pkgs.runCommandLocal "the-desktop-follows-dictation" { } ''
      cp ${../../dictation/Dictation.qml} Dictation.qml
      cp ${../../../tests/dictation.qml} shell.qml
      cp ${niri} Niri.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime
      export LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1
      export FONTCONFIG_FILE=${noFonts}
      mkdir -m 700 $XDG_RUNTIME_DIR
      timeout 120 ${lib.getExe pkgs.python3} ${../../../tests/dictation-watch.py} \
        ${qs}
      touch $out
    '';
}
// onLinux {
}
