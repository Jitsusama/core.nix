# mpv-stopped, which lists the files mpv stopped part way through for its
# Recent in the launcher; stopped.sh says how. ffprobe reads each file's
# length, from the ffmpeg yt-dlp already brings to mpv's wrapper.
{
  writeShellApplication,
  coreutils,
  findutils,
  gawk,
  gnused,
  ffmpeg-headless,
}:
writeShellApplication {
  name = "mpv-stopped";
  runtimeInputs = [
    coreutils
    findutils
    gawk
    gnused
    ffmpeg-headless
  ];
  text = builtins.readFile ./stopped.sh;
}
