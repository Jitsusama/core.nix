# The checks of camera, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{ onLinux, pkgs, ... }:
onLinux {
  # The camera program runs against a /dev and a /sys of its own making,
  # with an mpv that only says what it was asked to play: the machine's
  # camera with nothing plugged in, a camera on a removable USB port over
  # it, and never a fixed one, as a built-in USB camera's port says.
  the-camera-takes-the-one-plugged-in =
    pkgs.runCommandLocal "the-camera-takes-the-one-plugged-in" { nativeBuildInputs = [ pkgs.bash ]; }
      ''
        mkdir -p bin root/dev/v4l/by-id
        printf '#!%s\necho "$1"\n' "$BASH" > bin/mpv
        chmod +x bin/mpv
        echo /dev/video35 > made
        { echo 'device=made look= script='; cat ${./camera.sh}; } > camera
        plays() { CAMERA_ROOT=$PWD/root PATH=$PWD/bin:$PATH bash camera picture; }

        # Gives root a USB camera at video$1 whose port says $2.
        plug() {
          mkdir -p root/sys/devices/usb/port/interface$1 root/sys/class/video4linux/video$1
          echo "$2" > root/sys/devices/usb/port/removable
          ln -s ../../../devices/usb/port/interface$1 root/sys/class/video4linux/video$1/device
          touch root/dev/video$1
          ln -s ../../video$1 root/dev/v4l/by-id/usb-A_Camera_$1-video-index0
        }

        [ "$(plays)" = av://v4l2:/dev/video35 ] || { echo "with none plugged in: $(plays)"; exit 1; }
        plug 1 fixed
        [ "$(plays)" = av://v4l2:/dev/video35 ] || { echo "with a fixed one: $(plays)"; exit 1; }
        echo removable > root/sys/devices/usb/port/removable
        want=av://v4l2:$PWD/root/dev/v4l/by-id/usb-A_Camera_1-video-index0
        [ "$(plays)" = "$want" ] || { echo "with one plugged in: $(plays)"; exit 1; }
        touch $out
      '';

  # The viewfinder as camera.sh starts it, playing a test picture in place
  # of the camera and drawing nothing, beside a player script in mpv's own
  # folder that takes Z (stand-in-player.lua), as the player's vi-keys.lua
  # does: Escape and q leave it open, and ZZ closes it all the same
  # (keeps-its-keys.py).
  the-viewfinder-keeps-its-own-keys =
    pkgs.runCommandLocal "the-viewfinder-keeps-its-own-keys"
      {
        nativeBuildInputs = [
          pkgs.bash
          pkgs.python3
        ];
      }
      ''
        export HOME=$PWD XDG_RUNTIME_DIR=$PWD
        mkdir -p bin .config/mpv/scripts
        cp ${./stand-in-player.lua} .config/mpv/scripts/player.lua
        # What camera.sh asks for, but the camera a test picture, and the
        # window nowhere, answering on a socket.
        {
          printf '#!%s\nshift\n' "$BASH"
          printf 'exec %s av://lavfi:testsrc=size=64x48:rate=10 "$@"' ${pkgs.mpv}/bin/mpv
          printf ' --vo=null --ao=null --force-window=no --input-ipc-server=%s\n' "$PWD/mpv.sock"
        } > bin/mpv
        chmod +x bin/mpv
        { echo 'device=/dev/null look= script=${./camera.lua}'; cat ${./camera.sh}; } > camera
        CAMERA_ROOT=$PWD PATH=$PWD/bin:$PATH bash camera picture > mpv.log 2>&1 &
        viewfinder=$!
        # A viewfinder left open is stopped before what it said is shown,
        # since it says where it's playing as long as it plays.
        python3 ${./keeps-its-keys.py} mpv.sock || {
          kill $viewfinder
          wait $viewfinder || true
          grep -a -v '^V: ' mpv.log
          exit 1
        }
        wait $viewfinder
        touch $out
      '';
}
