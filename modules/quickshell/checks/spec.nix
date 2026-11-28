# The checks of the launcher's Spec, which the folder's checks.nix gathers
# with every other place's; its opening comment says why each place keeps
# its own.
{
  account,
  copied,
  graphical,
  launcherOnNiri,
  lib,
  noFonts,
  pkgs,
  qs,
  shellFolderLint,
  ...
}:
{
  # Spec reads what the cadence-spec daemon itself serves, run here over a
  # stand-in for cadence printing two axes, one out of spec with a reading
  # inside it (modules/cadence/tests/served-stand-in.sh): the head as the
  # daemon says it with the alert glyph, which Agents' Spec row shows, the
  # axes in the order served, each a step in, and the out axis's reading
  # inside it (modules/quickshell/tests/spec-served.qml).
  the-launcher-reads-spec-from-its-daemon =
    let
      spec = lib.getExe (pkgs.callPackage ../../cadence/spec/package.nix { });
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal "the-launcher-reads-spec-from-its-daemon" { } ''
      export HOME=$TMPDIR
      printf '#!%s\nexec %s %s "$@"\n' ${pkgs.runtimeShell} ${pkgs.runtimeShell} \
        ${../../cadence/tests/served-stand-in.sh} > cadence
      chmod +x cadence
      ${copied [
        "Daemon.qml"
        "Fuzzy.js"
        "Options.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../../cadence/Spec.qml} launcher/Spec.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/spec-served.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      mkdir -p runtime/daemons
      chmod 700 runtime
      export XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      CADENCE_SPEC=${spec} CADENCE=$PWD/cadence WORDS=${(import ../launcher/words.nix).file} \
        timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # Asked For reads what the cadence-spec daemon itself serves, run here over
  # a stand-in for cadence (modules/cadence/tests/asked-stand-in.sh), twice:
  # once printing two asks, one late, which it gives as served, warning; and
  # once failing, as cadence does with the asks list unreadable, which it
  # gives as the daemon's unread head with no rows. Both times it says it's
  # still asking before the daemon answers, and gives no answer of its own
  # once the daemon stops (modules/quickshell/tests/asked-served.qml).
  the-launcher-says-what-you-asked-for =
    let
      spec = lib.getExe (pkgs.callPackage ../../cadence/spec/package.nix { });
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal "the-launcher-says-what-you-asked-for" { } ''
      export HOME=$TMPDIR
      printf '#!%s\nexec %s %s "$@"\n' ${pkgs.runtimeShell} ${pkgs.runtimeShell} \
        ${../../cadence/tests/asked-stand-in.sh} > cadence
      chmod +x cadence
      ${copied [
        "Daemon.qml"
        "Fuzzy.js"
        "Options.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Seen.js"
        "launcher/words.json"
      ]}
      cp ${../../cadence/Asked.qml} launcher/Asked.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../tests/asked-served.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      mkdir -p runtime/daemons
      chmod 700 runtime
      export XDG_RUNTIME_DIR=$PWD/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      export CADENCE_SPEC=${spec} CADENCE=$PWD/cadence WORDS=${(import ../launcher/words.nix).file}
      for read in rows unread; do
        rm -f runtime/daemons/cadence-spec.sock
        ASKED_READ=$read timeout 60 ${qs} -p shell.qml
      done
      touch $out
    '';

  # On the whole launcher, over cadence-spec itself and a stand-in for cadence
  # printing a file the cases rewrite, Agents lists Spec once with the cursor
  # staying on it as the daemon's answer changes, and a step held in Spec takes
  # the rows the daemon serves after a restart (modules/quickshell/tests/
  # spec-on-niri-test.py). `spend-watch serve` gives Agents' other part,
  # Budgets, so a - out of Now is held to making neither again while what
  # each is made of stands, and Budgets to going while spend-watch is down.
  # Spec's place among the parts is held in Qt's own engine
  # (tests/ranking.qml); the order here is the cadence module's.
  agents-lists-spec-once-on-niri = launcherOnNiri {
    name = "agents-lists-spec-once-on-niri";
    tests = ../tests/spec-on-niri-test.py;
    given = [
      "CADENCE_SPEC=${lib.getExe (pkgs.callPackage ../../cadence/spec/package.nix { })}"
      "SPEC_ROWS=${../../cadence/tests/served-stand-in.sh}"
      "WORDS=${(import ../launcher/words.nix).file}"
      "SPEND_WATCH=${lib.getExe (pkgs.callPackage ../../spend-watch/package.nix { })}"
    ];
    shell.launcher.providers.spec = {
      order = 54;
      source = ../../cadence/Spec.qml;
    };
  };
}
