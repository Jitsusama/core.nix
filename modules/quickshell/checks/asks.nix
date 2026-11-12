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
  # agent puts one: For You holds each waiting one, Return releases it, a
  # yes while locked is refused in the daemon's words, and Refuse It
  # refuses the other (tests/asks-served.qml).
  the-asks-place-releases-through-its-daemon =
    let
      asks = pkgs.callPackage ../../secret-asks/package.nix { };
      item = "op://Private/Release Test/password";
      git = lib.getExe pkgs.git;
    in
    pkgs.runCommandLocal "the-asks-place-releases-through-its-daemon" { } ''
      mkdir -p runtime/daemons tree && chmod 700 runtime
      export HOME=$TMPDIR GIT_CONFIG_GLOBAL=/dev/null
      ${git} -C tree init -q
      echo "a tree" > tree/README
      ${git} -C tree add README
      ${git} -C tree -c user.name=t -c user.email=t@example.com commit -q -m "a commit"
      # op leaves a line each time it runs, and says the item holds a
      # value only when asked for the one item; loginctl says the session
      # is locked as the file beside it says.
      cat > op <<EOF
      #!${pkgs.runtimeShell}
      echo "\$@" >> $PWD/op-ran
      [ "\$3" = '${item}' ] || exit 1
      printf '%s' 'this-is-not-a-secret'
      EOF
      cat > loginctl <<EOF
      #!${pkgs.runtimeShell}
      case "\$1" in show-user) echo 2 ;; show-session) cat $PWD/locked ;; esac
      EOF
      chmod +x op loginctl
      echo no > locked
      ask() {
        ${lib.getExe asks} ask --item '${item}' --tree $PWD/tree --why "To try the line." \
          --asks $PWD/runtime/secret-asks
      }
      ASK_YES=$(ask)
      ASK_NO=$(ask)
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
      export XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      FOLDER=$PWD ITEM='${item}' ASK_YES=$ASK_YES ASK_NO=$ASK_NO GIT=${git} \
        SECRET_ASKS=${lib.getExe asks} WORDS=${(import ../launcher/words.nix).file} \
        timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
