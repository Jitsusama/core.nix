# What the harness's done-chime-widget rings with on Linux, as its
# sound.json names it: a loud, low ding, then a line piper speaks in a
# voice on the machine, lowered by sox, both played by PipeWire's pw-play.
# Joel asked for a loud ding and a spoken voice when pi is done and has
# something to propose, then for it lower, for his dog.
#
# The ding is struck here rather than taken from a sound theme: the
# freedesktop theme's bell, message and complete last a tenth of a second
# to a second and read at 840 Hz to 5 kHz, too slight and too high. A
# bell on G3, 196 Hz, sits far below where a dog's hearing is keenest and
# still high enough that a laptop's speakers carry it, its partials up to
# 1 kHz giving it its ring; it dies
# away over about two seconds, and its peak sits just under full scale,
# so it's heard from across the room. It is a different sound from the
# shell's soft one for a session wanting Joel, since it means something
# else: work he handed over is done.
#
# The voice is lowered three semitones, from about 100 Hz to 84.
{ pkgs, lib }:
let
  # Only what speaks: the training, alignment, HTTP and Japanese extras
  # bring PyTorch and its kin, near 4 GB, for nothing a line needs.
  piper =
    (pkgs.piper-tts.override {
      withAlignment = false;
      withHTTP = false;
      withJapanese = false;
      withTrain = false;
    }).overridePythonAttrs
      (_: {
        pythonImportsCheck = [ "piper" ];
      });
  voice = pkgs.callPackage ./voice.nix { };
  ding = pkgs.runCommand "pi-ding.wav" { nativeBuildInputs = [ pkgs.sox ]; } ''
    ${lib.getExe pkgs.bash} ${./ding.sh} $out
  '';
in
{
  ding = "${ding}";
  player = lib.getExe' pkgs.pipewire "pw-play";
  voice = "${voice}/en_GB-alan-medium.onnx";
  piper = lib.getExe piper;
  sox = lib.getExe' pkgs.sox "sox";
  pitch = -300;
}
