# The checks of the launcher in a terminal, and the say and chord programs
# beside it, which the folder's checks.nix gathers with every other place's; its
# opening comment says why each place keeps its own.
{
  lib,
  noFonts,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # What `launcher show` prints in a terminal is what the launcher holds:
  # a step names its row, a miss the names nearest it, and a row and its
  # drawer come as the words drawn, as tests/showing.qml holds
  # launcher/Showing.js to it in Qt's own engine.
  the-launcher-shows-a-terminal-what-it-holds =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-shows-a-terminal-what-it-holds" { } ''
      cp ${../../../tests/showing.qml} showing.qml
      cp ${../launcher/Showing.js} Showing.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml showing.qml
      touch $out
    '';

  # clippy finds nothing in launcher-say, its tests included.
  launcher-say-passes-clippy = passesClippy "launcher-say-passes-clippy" (
    pkgs.callPackage ../launcher-say/package.nix { }
  );

  # clippy finds nothing in shell-weighs, its tests included.
  shell-weighs-passes-clippy = passesClippy "shell-weighs-passes-clippy" (
    pkgs.callPackage ../shell-weighs/package.nix { }
  );

  # A chord's program asks the loader for no library, so a chord on a
  # loaded machine reads only its one file, which launcher-say-resident
  # holds in memory (launcher-say/src/main.rs).
  a-chord-reads-only-its-own-program =
    let
      say = pkgs.callPackage ../launcher-say/package.nix { };
    in
    # Kept in Nix because it is one readelf call over the built program,
    # with nothing in it to test alone.
    pkgs.runCommandLocal "a-chord-reads-only-its-own-program" { } ''
      if ${pkgs.binutils}/bin/readelf -l ${lib.getExe say} | grep -q INTERP; then
        echo "launcher-say asks for a loader, so it reads libraries as it starts:" >&2
        ${pkgs.binutils}/bin/readelf -d ${lib.getExe say} >&2
        exit 1
      fi
      touch $out
    '';

  # launcher's tests run as it builds: what the shell shows is laid out to
  # the terminal's width, a count and then a side giving way by whole
  # items and a name cut only at a word, inked only where the terminal
  # takes colour, and a miss said in the launcher's words.
  launcher-fits-what-it-shows-to-the-terminal = pkgs.callPackage ../in-a-terminal/package.nix { };

  # clippy finds nothing in launcher, the launcher in a terminal, its tests
  # included.
  launcher-passes-clippy = passesClippy "launcher-passes-clippy" (
    pkgs.callPackage ../in-a-terminal/package.nix { }
  );

  # launcher completes in zsh, typed in a real line editor with the real
  # launcher and the completion it writes, asking a stand-in qs what each
  # place holds: its command, each row a step names as it's typed, the
  # rows inside the steps typed so far, flags anywhere, and nothing after a
  # step that names nothing (in-a-terminal/tests/launcher-completion.zsh).
  launcher-completes-in-zsh =
    let
      launcher = pkgs.callPackage ../in-a-terminal/package.nix { };
    in
    pkgs.runCommandLocal "launcher-completes-in-zsh"
      {
        nativeBuildInputs = [
          launcher
          pkgs.zsh
        ];
        LAUNCHER_QS = pkgs.writeShellScript "qs" (builtins.readFile ../in-a-terminal/tests/qs);
        LAUNCHER_WORDS = (import ../launcher/words.nix).file;
      }
      ''
        export HOME=$PWD
        launcher completion zsh > completion.zsh
        zsh -f ${../in-a-terminal/tests/launcher-completion.zsh} $PWD/completion.zsh
        touch $out
      '';
}
