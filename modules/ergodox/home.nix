# The ErgoDox EZ, on every machine Joel plugs it into: its firmware, built
# from the layout beside this file, the `ergodox` command that writes it to
# the board and sends it commands, and zapp, ZSA's flasher, which that
# command runs. On Linux the
# board's USB devices need the permissions nixos.nix gives them; a Mac lets
# any program reach them.
{
  qmk,
  lufa,
  printf,
}:
{ pkgs, ... }:
let
  firmware = import ./firmware.nix pkgs {
    firmware = qmk;
    inherit lufa printf;
  };
in
{
  home.packages = [
    pkgs.zapp
    (pkgs.writeShellApplication {
      name = "ergodox";
      runtimeInputs = [
        pkgs.coreutils
        pkgs.gnugrep
        pkgs.zapp
      ];
      runtimeEnv = {
        ERGODOX_FIRMWARE = firmware;
        ERGODOX_LAYERS = toString (import ./layers.nix);
      };
      text = builtins.readFile ./ergodox.sh;
    })
  ];
}
