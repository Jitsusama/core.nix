# The checks of call-transcripts' shell half, which checks.nix gathers with
# every other folder's; its opening comment says what each kind of check
# proves. The program itself is dictation's, checked with it; its people
# source, transcripts-people, is this folder's crate.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  shellFolderLint,
  ...
}:
# The transcripts serve the Linux desktop's shell, and build only on Linux.
onLinux {
  # transcripts-people's tests run as it builds: each call with someone is
  # said as the thing they met at, named by the meeting, by who Tuple said
  # was on it, by their address or by when it began; its note says when
  # and how long, or that it's being written down; its acts open its
  # transcript or folder and copy a path; and its drawer sets its summary
  # or its opening from markdown, nothing it said read as markup. An
  # address is answered at once, in any case, and again as a call with it
  # changes or goes, over a stand-in of call-transcripts' socket.
  transcripts-people-says-each-call = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in transcripts-people, its tests included.
  transcripts-people-passes-clippy = passesClippy "transcripts-people-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

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
      # Home Manager keeps ExecStart as a list of its lines.
      sums = machine: lib.hasInfix "call-summarizer" (toString (service machine).ExecStart);
    in
    holds "calls-are-summed-up-only-where-the-model-runs"
      "call-transcripts is given a summarizer on a machine whose hardware names no device for it, or none on the XPS 14."
      (sums xps && lib.elem "SUMMARIZER_DEVICE=GPU" (service xps).Environment && !sums graphical);

  # How the shell says each call written down, when it was and for how
  # long, its drawer's words set from the markdown they're written in, and
  # which calls are with someone or of a meeting (tests/transcripts.qml,
  # over Transcripts.js and the When.js it reads).
  the-desktop-reads-the-transcripts =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-desktop-reads-the-transcripts" { } ''
      cp ${./tests/transcripts.qml} transcripts.qml
      cp ${./Transcripts.js} Transcripts.js
      cp ${../quickshell/When.js} When.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml transcripts.qml
      touch $out
    '';
}
