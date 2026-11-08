# The checks of search by meaning, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
# The daemon's own tests and clippy are dictation's, whose crate it's
# built in.
{
  account,
  graphical,
  holds,
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # A switch restarts it after it reloads the units, so the one answering
  # the shell is never the one from before the switch.
  meaning-switches-to-the-new-one =
    let
      unit = (account graphical).systemd.user.services.meaning.Unit;
    in
    holds "meaning-switches-to-the-new-one"
      "A switch stops meaning before reloading, so the shell would ask the old one."
      ((unit.X-SwitchMethod or null) == "restart");
}
// onLinux {
  # What the launcher works out to search by meaning, as tests/meaning.qml
  # holds it, in Qt's own engine.
  meaning-adds-only-below =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "meaning-adds-only-below" { } ''
      cp ${../../tests/meaning.qml} meaning.qml
      cp ${./Meaning.js} Meaning.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml meaning.qml
      touch $out
    '';

  # The daemon end to end, on the CPU, in the build's sandbox:
  # tests/a-thing-is-found-by-its-meaning.sh says what it proves.
  meaning-finds-a-thing-by-what-it-means =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      models = pkgs.callPackage ./models.nix { };
    in
    pkgs.runCommand "meaning-finds-a-thing-by-what-it-means"
      {
        nativeBuildInputs = [
          pkgs.jq
          pkgs.socat
        ];
      }
      ''
        $BASH ${./tests/a-thing-is-found-by-its-meaning.sh} ${dictation}/bin/meaning ${models}
        touch $out
      '';

  # The daemon's pictures end to end, on the CPU, in the build's sandbox:
  # tests/a-picture-is-found-by-what-it-shows.sh says what it proves.
  meaning-finds-a-picture-by-what-it-shows =
    let
      dictation = pkgs.callPackage ../dictation/package.nix { };
      models = pkgs.callPackage ./models.nix { };
      sight = pkgs.callPackage ./sight.nix { };
    in
    pkgs.runCommand "meaning-finds-a-picture-by-what-it-shows"
      {
        nativeBuildInputs = [
          pkgs.imagemagick
          pkgs.jq
          pkgs.socat
        ];
      }
      ''
        $BASH ${./tests/a-picture-is-found-by-what-it-shows.sh} \
          ${dictation}/bin/meaning ${models} ${sight}
        touch $out
      '';
}
