# The checks of the launcher's Sound and the music playing, which the folder's
# checks.nix gathers with every other place's; its opening comment says why
# each place keeps its own.
{
  account,
  copied,
  graphical,
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
      ${copied [
        "Daemon.qml"
        "NoiseCancelling.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Sound.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
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

  # Choose by Ear's steps follow the noise-cancelled daemon's trial record:
  # Record, its count, the clips heard blind with Return playing one and
  # Pick a step in, every cleaner and its cost once one's picked, a refusal
  # said plainly, the daemon not answering said, and leaving discarding
  # anything but idle, as tests/by-ear.qml holds launcher/ByEar.js to
  # records shaped as the trial protocol sends them.
  choose-by-ear-walks-its-steps =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "choose-by-ear-walks-its-steps" { } ''
      cp ${../../../tests/by-ear.qml} by-ear.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      ${copied [ "launcher/ByEar.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml by-ear.qml
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
      ${copied [ "Music.js" ]}
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
      ${copied [
        "Pictures.js"
        "launcher/Media.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml media.qml
      touch $out
    '';
}
// onLinux {
}
