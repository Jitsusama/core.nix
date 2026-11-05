# swayimg-command, which the launcher's command line runs to carry a
# command to the swayimg it was opened over; command.sh says how.
{ writeShellApplication, coreutils }:
writeShellApplication {
  name = "swayimg-command";
  runtimeInputs = [ coreutils ];
  text = builtins.readFile ./command.sh;
}
