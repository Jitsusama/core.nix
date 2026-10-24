# The checks of ergodox, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # Joel plugs the ErgoDox into every machine he writes code on, so every
  # account on one can flash it, and on Linux its devices are open to him.
  ergodox-on-every-workstation =
    let
      hasErgodox =
        machine: lib.any (package: lib.getName package == "ergodox") (account machine).home.packages;
    in
    holds "ergodox-on-every-workstation" "A workstation can't flash the ErgoDox." (
      hasErgodox workstation
      && hasErgodox macWorkstation
      && workstation.config.hardware.keyboard.zsa.enable
    );
}
// onLinux {
  # The ErgoDox's layout compiles into firmware that fits the board, and the
  # board will answer with the build's own serial number, never Oryx's, so
  # ZSA's tools don't take it for an Oryx layout and replace it. USB gives
  # a serial number in UTF-16.
  the-ergodox-layout-builds =
    let
      firmware = import ./firmware.nix pkgs {
        firmware = self.inputs.qmk;
        inherit (self.inputs) lufa printf;
      };
    in
    pkgs.runCommandLocal "the-ergodox-layout-builds"
      {
        nativeBuildInputs = [
          pkgs.pkgsCross.avr.buildPackages.binutils
          pkgs.python3
        ];
      }
      ''
        avr-objcopy -I ihex -O binary ${firmware}/ergodox.hex image
        python3 ${../../tests/ergodox-answers-with-its-serial.py} image ${firmware}/serial
        touch $out
      '';

  # The ergodox command finds the board's channel in a copy of sysfs by
  # ZSA's vendor number and Oryx's usage page together, and writes each
  # command as one report. Another vendor's raw channel and the board's
  # keyboard get nothing, and with no channel the command says so
  # (tests/ergodox-command-reaches-the-board.sh).
  the-ergodox-command-reaches-the-board =
    let
      ergodox = lib.findFirst (
        package: lib.getName package == "ergodox"
      ) (throw "No ergodox command.") (account workstation).home.packages;
    in
    pkgs.runCommandLocal "the-ergodox-command-reaches-the-board" { } ''
      ${lib.getExe pkgs.bash} ${../../tests/ergodox-command-reaches-the-board.sh} ${ergodox}/bin/ergodox
      touch $out
    '';

  # The layer watcher's own tests: it finds the board's channel as the
  # command does, asks to watch in one report, and says only what a watch
  # report tells it, by the layer's name.
  the-ergodox-watch-tells-the-layer =
    pkgs.runCommandCC "the-ergodox-watch-tells-the-layer" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./ergodox-watch.rs}
        ./tests
        touch $out
      '';
}
