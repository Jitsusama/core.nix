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
  ...
}:
{
  # Asks lists what the secret-asks daemon serves, run here over its own
  # folder with stand-ins for op and loginctl, and two asks put up as an
  # agent puts one (tests/asks-stand-ins.sh): For You holds each waiting
  # one, Return releases it once a first Return has armed it, r refuses,
  # a yes while locked is refused in the daemon's words, and Refuse It
  # refuses the other (tests/asks-served.qml).
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
      read -r ASK_YES ASK_NO < <(${pkgs.runtimeShell} ${../../../tests/asks-stand-ins.sh} \
        $PWD '${item}' ${asks} ${git} ${pkgs.runtimeShell})
      ${copied [
        "Daemon.qml"
        "Fuzzy.js"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../../secret-asks/Asks.qml} launcher/Asks.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../../../tests/asks-served.qml} shell.qml
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
