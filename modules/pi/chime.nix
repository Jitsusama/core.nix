# What the harness's done-chime-widget rings with on Linux, as its
# sound.json names it: a loud, low ding, played by PipeWire's pw-play.
# Joel asked for a loud ding when pi is done and has something to
# propose, then for it lower, for his dog. A voice saying which session
# came with it until 9 November, when he kept only the bell; a machine
# that wants a voice names one in the harness's own sound.json.
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
{ pkgs, lib }:
let
  ding = pkgs.runCommand "pi-ding.wav" { nativeBuildInputs = [ pkgs.sox ]; } ''
    ${lib.getExe pkgs.bash} ${./ding.sh} $out
  '';
in
{
  ding = "${ding}";
  player = lib.getExe' pkgs.pipewire "pw-play";
}
