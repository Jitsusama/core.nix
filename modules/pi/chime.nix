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
# so it's heard from across the room.
#
# `want` is what the shell plays when a session first stops on Joel's keys,
# a question or a yes: "make
# sure you make some loud dinging noises when you need me" (16 October).
# It had been the freedesktop theme's message, the high sound that scared
# his dog. It is the same bell struck twice and damped sooner, so it reads
# as a call rather than the done ding's ending, which means something
# else: work he handed over is done.
{ pkgs, lib }:
let
  struck =
    name: script:
    pkgs.runCommand name { nativeBuildInputs = [ pkgs.sox ]; } ''
      ${lib.getExe pkgs.bash} ${script} $out
    '';
in
{
  ding = "${struck "pi-ding.wav" ./ding.sh}";
  want = "${struck "pi-want.wav" ./want.sh}";
  player = lib.getExe' pkgs.pipewire "pw-play";
}
