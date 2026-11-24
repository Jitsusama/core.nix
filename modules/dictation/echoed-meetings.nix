# The meetings of meetings.nix as a call on speakers sounds: the far end,
# as it leaves Joel's speakers, heard again by his microphone 50 ms late
# and 20 dB down, mixed into his side. That's the echo a call with no
# echo cancelling hears, so `call-transcripts score` on this set measures
# how much of it is written as Joel's words, and how many of his own go
# with it. The far end's side and the references are the meetings' own
# (echo-meetings.sh mixes them).
{
  bash,
  ffmpeg-headless,
  lib,
  meetings,
  runCommand,
}:
runCommand "call-transcripts-echoed-meetings" { nativeBuildInputs = [ ffmpeg-headless ]; } ''
  ${lib.getExe bash} ${./echo-meetings.sh} ${meetings} $out
''
