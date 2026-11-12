# What GLib starts each program through, giving it a scope of its own
# (nixos.nix says why).
{ systemd, writeShellApplication }:
writeShellApplication {
  name = "launched";
  runtimeInputs = [ systemd ];
  text = builtins.readFile ./launched.sh;
}
