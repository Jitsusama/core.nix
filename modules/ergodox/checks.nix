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
        python3 - ${firmware}/serial <<'EOF'
        import sys
        image = open("image", "rb").read()
        serial = open(sys.argv[1]).read().strip()
        if serial.encode("utf-16-le") not in image:
            sys.exit(f"The board wouldn't answer with the build's serial, {serial}.")
        if "omjyr".encode("utf-16-le") in image:
            sys.exit("The board would still answer with Oryx's serial.")
        EOF
        touch $out
      '';

  # The ergodox command finds the board's channel in a copy of sysfs by
  # ZSA's vendor number and Oryx's usage page together, and writes each
  # command as one report. Another vendor's raw channel and the board's
  # keyboard get nothing, and with no channel the command says so.
  the-ergodox-command-reaches-the-board =
    let
      ergodox = lib.findFirst (
        package: lib.getName package == "ergodox"
      ) (throw "No ergodox command.") (account workstation).home.packages;
    in
    pkgs.runCommandLocal "the-ergodox-command-reaches-the-board" { } ''
      node() {
        mkdir -p root/sys/class/hidraw/$1/device root/dev
        echo "HID_ID=$2" > root/sys/class/hidraw/$1/device/uevent
        printf "$3" > root/sys/class/hidraw/$1/device/report_descriptor
        : > root/dev/$1
      }
      node hidraw0 0003:0000046D:0000C52B '\006\140\377\011\141'
      node hidraw1 0003:00003297:00004976 '\005\001\011\006'
      node hidraw2 0003:00003297:00004976 '\006\140\377\011\141'
      export ERGODOX_ROOT=$PWD/root

      sent() { od -An -tx1 -v root/dev/$1 | tr -d ' \n'; }
      expect() {
        ${ergodox}/bin/ergodox $1 $2
        want=00$3$(printf '%060d' 0)
        [ "$(sent hidraw2)" = "$want" ] || {
          echo "ergodox $1 $2 wrote $(sent hidraw2), not $want."
          exit 1
        }
        [ -z "$(sent hidraw0)$(sent hidraw1)" ] || {
          echo "ergodox $1 $2 wrote to a node that isn't the board's channel."
          exit 1
        }
      }
      expect bootloader "" 8000
      expect layer numb 8202
      expect layer Punk 8201
      expect layer 0 8200
      expect lights off 8300
      expect brighter "" 8401
      expect dimmer "" 8400

      status=0
      ${ergodox}/bin/ergodox layer sideways 2> /dev/null || status=$?
      [ $status = 64 ] || { echo "An unknown layer exited $status, not 64."; exit 1; }

      rm -r root/sys/class/hidraw/hidraw2 root/dev/hidraw2
      status=0
      ${ergodox}/bin/ergodox bootloader 2> /dev/null || status=$?
      [ $status = 69 ] || { echo "With no board, bootloader exited $status, not 69."; exit 1; }
      [ -z "$(sent hidraw0)$(sent hidraw1)" ] || {
        echo "With no board, ergodox wrote to another node."
        exit 1
      }
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
