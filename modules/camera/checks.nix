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
}
