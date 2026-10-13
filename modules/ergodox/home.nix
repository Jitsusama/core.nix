# The ErgoDox EZ, on every machine Joel plugs it into: its firmware, built
# from the layout beside this file, the `ergodox` command that writes it to
# the board and sends it commands, zapp, ZSA's flasher, which that command
# runs, and the layout described for the shell, in ergodox/layout.json among
# the account's settings. On Linux the
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
  xdg.configFile."ergodox/layout.json".source = "${firmware}/shell.json";

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
