# The checks of the launcher's Spec, which the folder's checks.nix gathers
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
  # Spec reads what the cadence-spec daemon itself serves, run here over a
  # stand-in for cadence printing two axes, one out of spec with a reading
  # inside it (../../cadence/tests/served-stand-in.sh): the head as the
  # daemon says it with the alert glyph, which Agents' Spec row shows, the
  # axes in the order served, each a step in, and the out axis's reading
  # inside it (tests/spec-served.qml).
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
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/words.json"
      ]}
      cp ${../../cadence/Spec.qml} launcher/Spec.qml
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../../../tests/spec-served.qml} shell.qml
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
}
