# The meetings of meetings.nix as a call on speakers sounds: the far end,
# as it leaves Joel's speakers, heard again by his microphone 50 ms late
# and 20 dB down, mixed into his side. That's the echo a call with no
# echo cancelling hears, so `call-transcripts score` on this set measures
# how much of it is written as Joel's words, and how many of his own go
# with it. The far end's side and the references are the meetings' own.
{
  ffmpeg-headless,
  meetings,
  runCommand,
}:
runCommand "call-transcripts-echoed-meetings" { nativeBuildInputs = [ ffmpeg-headless ]; } ''
  mkdir $out
  cp ${meetings}/sides ${meetings}/*.ref ${meetings}/*.theirs.wav $out/
  while read -r name _; do
    [ -n "$name" ] || continue
    ffmpeg -nostdin -loglevel error -i ${meetings}/$name.mine.wav -i ${meetings}/$name.theirs.wav \
      -filter_complex "[1]adelay=50,volume=-20dB[e];[0][e]amix=inputs=2:duration=first:normalize=0" \
      -ar 16000 -ac 1 -c:a pcm_s16le $out/$name.mine.wav
  done < ${meetings}/sides
''
