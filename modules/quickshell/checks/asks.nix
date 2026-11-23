# The checks of the launcher's Asks, which the folder's checks.nix gathers
# with every other place's; its opening comment says why each place keeps
# its own.
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
let
  # What Asks reads of niri and the recorder to hide an ask while the
  # screen is shared (launcher/Shared.js): no cast, and niri having said so,
  # unless a case says otherwise.
  niri = singleton "Niri.qml" ''
    property var casts: ({})
    property bool castsHeard: true
  '';
  recorder = singleton "Recorder.qml" "property var windowSession: null";
in
{
  # A yes says how long its line was on the screen, from the first frame niri
  # presented it in this opening, and nothing where it wasn't, where the line changed
  # since, or where the launcher is walked from outside; opened from outside, a line
  # is timed only from a key that moves over the rows, typing starts it again and
  # Return never does, as modules/quickshell/tests/asks-shown.qml holds
  # secret-asks/Shown.js to asks made up there.
  the-asks-say-how-long-each-was-shown =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-asks-say-how-long-each-was-shown" { } ''
      mkdir launcher
      cp ${../tests/asks-shown.qml} asks-shown.qml
      cp ${../../secret-asks/Shown.js} launcher/Shown.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml asks-shown.qml
      touch $out
    '';

  # Asks over Passwords' connection, as where the passwords daemon answers
  # secret asks, stood in so each ask it sends is read
  # (modules/quickshell/tests/asks-over.qml): a first Return only arms; a yes
  # says how long since a frame presented its line selected, its line noted
  # again as the launcher opens on it; a changed line says no time until
  # presented; opened from outside, it times nothing until a key moves over the
  # rows his Escape reached, never on the find, from a step into a place, nor
  # over rows a landing Return, the command line or an ask from outside left,
  # and a withheld yes says why; entries told over the connection leave the
  # asks' rows be; a yes walked from outside is never sent; a no goes as served;
  # and while the screen is shared, or niri hasn't said its casts, a row is
  # hidden, with no act and no frame timed, counting nothing once shown until
  # his next move.
  the-asks-say-how-long-each-was-shown-over-passwords =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal "the-asks-say-how-long-each-was-shown-over-passwords" { } ''
      export HOME=$TMPDIR
      ${copied [
        "Arming.js"
        "Daemon.qml"
        "Fuzzy.js"
        "Options.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Shared.js"
        "launcher/words.json"
      ]}
      cp ${../../secret-asks/Asks.qml} launcher/Asks.qml
      cp ${../../secret-asks/Shown.js} launcher/Shown.js
      cp ${niri} Niri.qml
      cp ${recorder} Recorder.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/asks-over.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      export XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Asks lists what the secret-asks daemon serves, run here over its own folder
  # with stand-ins for op and loginctl, and two asks put up as an agent puts one
  # (modules/quickshell/tests/asks-stand-ins.sh): For You holds each waiting
  # one, Return releases it once a first Return has armed it, r refuses, a yes
  # while locked is refused in the daemon's words, and Refuse It refuses the
  # other (modules/quickshell/tests/asks-served.qml).
  the-asks-place-releases-through-its-daemon =
    let
      asks = lib.getExe (pkgs.callPackage ../../secret-asks/package.nix { });
      item = "op://Private/Release Test/password";
      git = lib.getExe pkgs.git;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal "the-asks-place-releases-through-its-daemon" { } ''
      export HOME=$TMPDIR GIT_CONFIG_GLOBAL=/dev/null
      read -r ASK_YES ASK_NO < <(${pkgs.runtimeShell} ${../tests/asks-stand-ins.sh} \
        $PWD '${item}' ${asks} ${git} ${pkgs.runtimeShell})
      ${copied [
        "Daemon.qml"
        "Fuzzy.js"
        "Options.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Shared.js"
        "launcher/words.json"
      ]}
      cp ${../../secret-asks/Asks.qml} launcher/Asks.qml
      cp ${../../secret-asks/Shown.js} launcher/Shown.js
      cp ${niri} Niri.qml
      cp ${recorder} Recorder.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/asks-served.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      export XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      FOLDER=$PWD ITEM='${item}' ASK_YES=$ASK_YES ASK_NO=$ASK_NO GIT=${git} \
        SECRET_ASKS=${asks} WORDS=${(import ../launcher/words.nix).file} \
        timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
