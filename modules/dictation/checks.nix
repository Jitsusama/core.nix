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

  # A call's two sides, each its own recording from the call's start, come
  # out as one transcript in the order they spoke: Joel asks, the far end
  # answers in another voice while his side is quiet, and Joel answers
  # back. Each side's pauses cut it into takes, so three paragraphs, the
  # middle one the others', prove the cutting, the times and the order.
  call-transcripts-writes-down-both-sides =
    let
      dictation = pkgs.callPackage ./package.nix { };
      model = pkgs.callPackage ./model.nix { };
    in
    pkgs.runCommand "call-transcripts-writes-down-both-sides"
      {
        nativeBuildInputs = [
          pkgs.espeak-ng
          pkgs.sox
        ];
      }
      ''
        say() {
          espeak-ng -v "$1" -s 160 -w said.wav "$2"
          sox said.wav -r 16000 -c 1 -b 16 "$3"
        }
        say en-us "Can we move the review to Thursday?" ask.wav
        say en-gb "Thursday morning works for everyone." answer.wav
        say en-us "Perfect, thank you so much." thanks.wav
        sox ask.wav asked.wav pad 0.5 6
        sox asked.wav thanks.wav mine.wav
        sox answer.wav theirs.wav pad 4 4
        DICTATION_DEVICE=CPU ${dictation}/bin/call-transcripts transcribe ${model} \
          mine.wav theirs.wav > transcript.md
        cat transcript.md
        grep '^\*\*' transcript.md > said
        if [ "$(wc -l < said)" != 3 ]; then
          echo "the transcript doesn't have three turns"
          exit 1
        fi
        turn() { sed -n "$1p" said; }
        if turn 1 | grep -q '^\*\*Others\*\*' || ! turn 1 | grep -qi review; then
          echo "the first turn isn't Joel's question"
          exit 1
        fi
        if ! turn 2 | grep -q '^\*\*Others\*\*, 0:0[34]: ' || ! turn 2 | grep -qi morning; then
          echo "the second turn isn't the others' answer, about four seconds in"
          exit 1
        fi
        if turn 3 | grep -q '^\*\*Others\*\*' || ! turn 3 | grep -qi thank; then
          echo "the last turn isn't Joel's answer"
          exit 1
        fi
        touch $out
      '';

  # A call writes down only what was said. Joel's side is a quiet room, a
  # second of loud noise with no speech in it, then his question; the far
  # end's is silence with a click. Dictation's words are set, as they are
  # on a machine that has them, since Whisper told them writes them back
  # for a take with no speech, as a real call's did. One turn, Joel's
  # question, with none of those words and no "[static]" for the noise,
  # proves the noise and the silence gave none.
  call-transcripts-writes-only-what-was-said =
    let
      dictation = pkgs.callPackage ./package.nix { };
      model = pkgs.callPackage ./model.nix { };
      words = pkgs.writeText "dictation-words" "Quorvex\nTallimander\n";
    in
    pkgs.runCommand "call-transcripts-writes-only-what-was-said"
      {
        nativeBuildInputs = [
          pkgs.espeak-ng
          pkgs.sox
        ];
      }
      ''
        espeak-ng -v en-us -s 160 -w said.wav "Can we move the review to Thursday?"
        sox said.wav -r 16000 -c 1 -b 16 ask.wav
        made() { sox -R -n -r 16000 -c 1 -b 16 "$@"; }
        made room.wav synth 4 pinknoise vol 0.001
        made noise.wav synth 1 pinknoise vol 0.03
        made click.wav synth 0.02 square 1000 vol 0.3
        made silence.wav trim 0 5
        sox room.wav noise.wav room.wav ask.wav room.wav mine.wav
        sox silence.wav click.wav silence.wav silence.wav theirs.wav
        DICTATION_DEVICE=CPU DICTATION_WORDS=${words} ${dictation}/bin/call-transcripts \
          transcribe ${model} mine.wav theirs.wav > transcript.md
        cat transcript.md
        if grep -qiE 'quorvex|tallimander' transcript.md; then
          echo "Whisper wrote dictation's words, which nobody said"
          exit 1
        fi
        if grep -qE '[][()]' transcript.md; then
          echo "the transcript says what the noise sounded like, which nobody said"
          exit 1
        fi
        grep '^\*\*' transcript.md > said
        if [ "$(wc -l < said)" != 1 ] || grep -q '^\*\*Others\*\*' said || ! grep -qi review said; then
          echo "the transcript isn't Joel's question alone"
          exit 1
        fi
        touch $out
      '';

  # screen-text reads a screenshot's words: a picture drawn as the check
  # builds, wider and taller than the detector's tile, holds a heading,
  # then a row of two lines apart, the first across where two tiles
  # overlap. Read on the CPU, as a machine with no NPU reads it, each line
  # must come back whole, in reading order.
  screen-text-reads-a-screenshot =
    let
      dictation = pkgs.callPackage ./package.nix { };
      models = pkgs.callPackage ../screen-text/models.nix { };
      font = "${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSans.ttf";
      want = pkgs.writeText "screen-text-wants" ''
        Quarterly planning review
        Move the review to Thursday morning at ten
        Owner: Joel
      '';
    in
    pkgs.runCommand "screen-text-reads-a-screenshot" { nativeBuildInputs = [ pkgs.imagemagick ]; } ''
      magick -size 1600x900 xc:white -font ${font} -fill black \
        -pointsize 40 -annotate +80+120 'Quarterly planning review' \
        -pointsize 24 -annotate +80+220 'Move the review to Thursday morning at ten' \
        -annotate +1100+220 'Owner: Joel' \
        screenshot.png
      SCREEN_TEXT_DEVICE=CPU ${dictation}/bin/screen-text read ${models} screenshot.png > read.txt
      if ! diff -u ${want} read.txt; then
        echo "screen-text read the screenshot wrong"
        exit 1
      fi
      touch $out
    '';

  # clippy finds nothing in dictation, its tests included.
  dictation-passes-clippy = passesClippy "dictation-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
