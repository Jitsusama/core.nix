# mpv-command, which the launcher's command line runs to carry a command
# to the mpv it was opened over; command.sh says how.
{
  writeShellApplication,
  jq,
  socat,
}:
writeShellApplication {
  name = "mpv-command";
  runtimeInputs = [
    jq
    socat
  ];
  text = builtins.readFile ./command.sh;
}
