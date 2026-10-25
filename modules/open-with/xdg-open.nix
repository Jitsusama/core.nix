# xdg-open, handing what it's given to gio (nixos.nix says why).
{ glib, writeShellApplication }:
writeShellApplication {
  name = "xdg-open";
  runtimeInputs = [ glib.bin ];
  text = builtins.readFile ./xdg-open.sh;
}
