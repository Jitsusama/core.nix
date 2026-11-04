# The checks of call-transcripts' shell half, which checks.nix gathers with
# every other folder's; its opening comment says what each kind of check
# proves. The program itself is dictation's, checked with it.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
# The transcripts serve the Linux desktop's shell, and build only on Linux.
onLinux {
  # Only a machine whose hardware names a device for the summarizer sums
  # calls up, on that device; any other graphical machine is given no
  # summarizer, so never fetches its 15 GB model.
  calls-are-summed-up-only-where-the-model-runs =
    let
      service = machine: (account machine).systemd.user.services.call-transcripts.Service;
      xps = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.dell-xps-14-da14260
      ];
      sums = machine: lib.hasInfix "call-summarizer" (service machine).ExecStart;
    in
    holds "calls-are-summed-up-only-where-the-model-runs"
      "call-transcripts is given a summarizer on a machine whose hardware names no device for it, or none on the XPS 14."
      (sums xps && lib.elem "SUMMARIZER_DEVICE=GPU" (service xps).Environment && !sums graphical);

  # How the shell says each call written down, when it was and for how
  # long, its drawer's words set from the markdown they're written in, and
  # which calls are with someone or of a meeting (tests/transcripts.qml,
  # over Transcripts.js).
  the-shell-reads-the-transcripts =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-shell-reads-the-transcripts" { } ''
      cp ${../../tests/transcripts.qml} transcripts.qml
      cp ${./Transcripts.js} Transcripts.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml transcripts.qml
      touch $out
    '';
}
