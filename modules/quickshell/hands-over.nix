# Hands the shell over to a new one as a switch reloads it
# (./shell-hands-over.sh), given the shell's command line. The unit runs it
# (home.nix), and the nested niri check runs the same script.
{ pkgs }:
pkgs.writeShellApplication {
  name = "quickshell-hands-over";
  runtimeInputs = [
    pkgs.coreutils
    pkgs.jq
    pkgs.procps
    pkgs.systemd
  ];
  text = builtins.readFile ./shell-hands-over.sh;
}
