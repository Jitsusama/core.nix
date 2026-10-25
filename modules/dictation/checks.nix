# The checks of dictation, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
{
  # A switch restarts dictation after it reloads the units, so the service
  # the shell's watch reaches is never the one from before the switch.
  dictation-switches-to-the-new-one =
    holds "dictation-switches-to-the-new-one"
      "A switch stops dictation before reloading, so the shell's watch starts the old one again."
      (((account graphical).systemd.user.services.dictation.Unit.X-SwitchMethod or null) == "restart");

  # Holding to talk needs the chord to run `dictation hold`, whose release
  # dictation-chord hears; a bind back on `toggle` would only toggle.
  dictation-stops-by-its-chord =
    let
      bound =
        lib.any (b: lib.hasInfix ''spawn "dictation" "hold";'' b.action)
          (account graphical).jitsusama.niri.binds;
    in
    holds "dictation-stops-by-its-chord"
      "No chord runs dictation hold, so dictation can't be held to talk."
      bound;
}
# Dictation runs on OpenVINO and its surface in Quickshell, both only on
# Linux, so a Mac doesn't evaluate these.
// onLinux {
  # What dictation's surface says in each state, its wave and its count
  # down, as tests/speaking.qml holds them, in Qt's own engine.
  dictation-says-what-its-doing =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "dictation-says-what-its-doing" { } ''
      cp ${../../tests/speaking.qml} speaking.qml
      cp ${./Speaking.js} Speaking.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml speaking.qml
      touch $out
    '';

  # dictation's tests run as it builds: its spectrogram matches the one that
  # transcribed a clip word for word, its transform the plain sum, its
  # vocabulary reads back into text and words into tokens, and silence or a
  # moment isn't heard.
  dictation-works-it-out = pkgs.callPackage ./package.nix { };

  # The whole of dictation's Whisper, on the CPU as a machine with no NPU
  # runs it: whisper.cpp's public clip, read and typed back word for word.
  dictation-hears-the-clip =
    let
      dictation = pkgs.callPackage ./package.nix { };
      model = pkgs.callPackage ./model.nix { };
      clip = "${pkgs.whisper-cpp.src}/samples/jfk.wav";
      want = lib.concatStringsSep " " [
        "And so my fellow Americans, ask not what your country can do for you,"
        "ask what you can do for your country."
      ];
    in
    pkgs.runCommand "dictation-hears-the-clip" { } ''
      heard=$(DICTATION_DEVICE=CPU ${lib.getExe dictation} transcribe ${model} ${clip})
      if [ "$heard" != ${lib.escapeShellArg want} ]; then
        echo "dictation heard: $heard"
        exit 1
      fi
      touch $out
    '';

  # Whisper writes a name it reads in the words file as it's spelled there.
  # espeak-ng says a sentence with Penetanguishene in it, which Whisper
  # alone hears as "Pennett and Wishing"; so the name missing without the
  # words proves the clip needs them, and present with them, that they
  # reach the decoder.
  dictation-writes-the-words-it-knows =
    let
      dictation = pkgs.callPackage ./package.nix { };
      model = pkgs.callPackage ./model.nix { };
      name = "Penetanguishene";
      words = pkgs.writeText "dictation-words" "${name}\n";
    in
    pkgs.runCommand "dictation-writes-the-words-it-knows"
      {
        nativeBuildInputs = [
          pkgs.espeak-ng
          pkgs.sox
        ];
      }
      ''
        espeak-ng -v en-us -s 160 -w said.wav "The drive from ${name} takes about three hours."
        sox said.wav -r 16000 -c 1 -b 16 clip.wav
        export DICTATION_DEVICE=CPU
        alone=$(${lib.getExe dictation} transcribe ${model} clip.wav)
        echo "without the words: $alone"
        if echo "$alone" | grep -q ${name}; then
          echo "Whisper knew the name without the words, so this proves nothing"
          exit 1
        fi
        knowing=$(DICTATION_WORDS=${words} ${lib.getExe dictation} transcribe ${model} clip.wav)
        echo "with the words: $knowing"
        if ! echo "$knowing" | grep -q ${name}; then
          echo "Whisper didn't write the name it was given"
          exit 1
        fi
        touch $out
      '';

  # clippy finds nothing in dictation, its tests included.
  dictation-passes-clippy = passesClippy "dictation-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
