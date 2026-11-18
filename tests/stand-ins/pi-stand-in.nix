# pi, as the shell opens it, for the VM tests, which have no pi: the
# program is pi-stand-in.sh beside it.
{ pkgs }:
pkgs.writeShellApplication {
  name = "pi";
  text = builtins.readFile ./pi-stand-in.sh;
}
