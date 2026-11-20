# The meetings call-transcripts' hearing is measured on: the first ten
# minutes of three of the AMI Meeting Corpus's test meetings (CC BY 4.0, by
# the AMI Consortium), each speaker on a headset of their own, every word
# written down by hand (tests/meetings, from AMI's manual annotations 1.6.2).
#
# Each is laid out as a call: one speaker's headset is Joel's side, and the
# other three, mixed, are the far end's, sent through Opus at 24 kbit/s as
# Meet sends a voice, each at its own level. A headset hears the whole
# room, so each is turned down 30 dB wherever its own speaker isn't
# talking, as the reference times it: otherwise the far end would hear
# Joel's side, which a call's never does, and every word of his would be
# counted twice. That stands in for what a call does already, Meet's noise
# suppression on the far end and the distance between rooms on Joel's, and
# it leaves room tone and the others 30 dB down rather than silence, so the
# cutter still finds its own pauses; it does find them more easily than in
# a noisy room, which a reference call with real Meet sound measures.
#
# A folder holds each meeting's two sides and its reference, and `sides`
# names whose voice is Joel's side in each; the others' are the far end.
# `call-transcripts score` reads it.
{
  fetchurl,
  ffmpeg-headless,
  lib,
  runCommand,
  sox,
}:
let
  # Where each meeting's headsets are, and whose is Joel's side: the
  # speaker who says the most in the first ten minutes, as Joel does on
  # his calls, save TS3003a's, which is a talk the far end gives.
  meetings = {
    ES2004a = {
      mine = "B";
      headsets = [
        "sha256-q7SRdxnG4DOwZvvUALbTf+Rt3WLIm1E/dwFqUfVOKtE="
        "sha256-m+joTk75Pqfu9mlHDNbbAXhW849zIeAFQZmtc5OhGZw="
        "sha256-pPL46n2WXt4sAQ42v8McFMd5LQdBm9W8lRnI3dHIvCg="
        "sha256-Wzz1mBbVFg13/wAxXcf224FsF17bZeDWFpFnn65jNXI="
      ];
    };
    IS1009a = {
      mine = "A";
      headsets = [
        "sha256-Y09CIH5Iiw6rOzSoH/FtVlcF+PxeN758w8O1ZheuMlc="
        "sha256-25jRdd2g0HYTeVxxVmenK5nFNyqr0U7k3LlNH6nCc+4="
        "sha256-nii/1VcnsH1qYCC3vyeX0ROzXGmho60lzEFQvgk3vyg="
        "sha256-egsEGtLyzZrxyPzuCOWY1tiL2Pd6MXTX654oGjDKBBM="
      ];
    };
    TS3003a = {
      mine = "B";
      headsets = [
        "sha256-PAJ8PMmeQ5qcCFCyLFFIZ08VXzLOF4tDpp1KOb4osXM="
        "sha256-izUm3b27r70oXVLAgyI5XnnjtxXOkNa3SaXXEiQMtR8="
        "sha256-7HaOo90q4aKM44P8NaX2RVaZNBbcKOTS2H6pEW7opcY="
        "sha256-cFW33aesLVpaM257XX5wCSd98hd00qBjPdl3Kwjqqsw="
      ];
    };
  };
  # AMI's speakers A to D wear headsets 0 to 3 in each of these meetings.
  speakers = [
    "A"
    "B"
    "C"
    "D"
  ];
  # How much of the sound around a speaker's own words stays loud.
  around = 0.3;
  # An ffmpeg filter keeping `speaker`'s headset loud only while the
  # reference has them talking, and 30 dB down the rest of the time.
  ownVoice =
    name: speaker:
    let
      said = lib.filter (line: line != "" && !lib.hasPrefix "#" line) (
        lib.splitString "\n" (builtins.readFile ./tests/meetings/${name}.ref)
      );
      fields = map (lib.splitString " ") said;
      theirs = lib.filter (line: lib.elemAt line 2 == speaker) fields;
      seconds = text: builtins.fromJSON text;
      span =
        line:
        "between(t,${toString (seconds (lib.elemAt line 0) - around)},${
          toString (seconds (lib.elemAt line 1) + around)
        })";
      talking = if theirs == [ ] then "0" else lib.concatMapStringsSep "+" span theirs;
    in
    "volume=volume='if(${talking},1,0.0316)':eval=frame";
  headset =
    name: i: hash:
    fetchurl {
      url = lib.concatStrings [
        "https://groups.inf.ed.ac.uk/ami/AMICorpusMirror/amicorpus/"
        "${name}/audio/${name}.Headset-${toString i}.wav"
      ];
      # The mirror answers 503 to many fetches at once, and the twelve are
      # fetched at once, so each keeps trying, waiting longer each time.
      curlOptsList = [
        "--retry"
        "10"
      ];
      inherit hash;
    };
  laid =
    name:
    { mine, headsets }:
    let
      files = lib.imap0 (headset name) headsets;
      voice = lib.listToAttrs (lib.zipListsWith lib.nameValuePair speakers files);
      others = lib.remove mine speakers;
      own = speaker: ''
        ffmpeg -loglevel error -i ${voice.${speaker}} -t 600 -af "${ownVoice name speaker}" \
          -c:a pcm_s16le ${speaker}.wav
      '';
    in
    ''
      ${lib.concatMapStrings own speakers}
      mv ${mine}.wav $out/${name}.mine.wav
      # Each voice at its own level, since sox would turn each of three
      # down to a third, and repeatably, since its dither is otherwise
      # seeded anew each run.
      sox -R -m ${lib.concatMapStringsSep " " (speaker: "-v 1 ${speaker}.wav") others} far.wav
      rm ${lib.concatMapStringsSep " " (speaker: "${speaker}.wav") others}
      ffmpeg -loglevel error -i far.wav -c:a libopus -b:a 24k -application voip far.opus
      ffmpeg -loglevel error -i far.opus -ar 16000 -ac 1 -c:a pcm_s16le $out/${name}.theirs.wav
      rm far.wav far.opus
      cp ${./tests/meetings/${name}.ref} $out/${name}.ref
      echo "${name} ${mine}" >> $out/sides
    '';
in
runCommand "call-transcripts-meetings"
  {
    nativeBuildInputs = [
      ffmpeg-headless
      sox
    ];
  }
  ''
    mkdir $out
    ${lib.concatStrings (lib.mapAttrsToList laid meetings)}
  ''
