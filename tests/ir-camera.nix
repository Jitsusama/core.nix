# Relays a stand-in IR camera, a loopback sent 16-bit greyscale frames of a
# known pattern, to the camera programs open. What has to hold shows only with
# the devices there: the camera stays closed until a program reads, the reader
# gets the pattern in 8-bit grey, the camera closes when it stops, and no reader
# gets a frame from before it opened, the last one's face least of all, and a
# relay whose loopback has gone waits for it rather than failing over and over.
# The script is ir-camera-vm-test.py.
{ self, pkgs }:
let
  width = 648;
  height = 368;

  # Sends the pattern to the loopback it's given, over and over, as a camera
  # would, and says when the first frame is out. It holds the loopback open but
  # sends nothing while /run/stand-in-paused exists, as a relay waiting for its
  # next reader does. The pattern is 10-bit, sent
  # as Y16, since v4l2loopback sizes Y10 frames at ten bits a pixel rather
  # than the sixteen they're stored in.
  camera = pkgs.writers.writePython3Bin "stand-in-camera" { } ''
    import fcntl
    import os
    import struct
    import sys
    import time

    VIDIOC_S_FMT = 0xC0D05605
    OUTPUT = 2
    Y16 = 0x20363159
    WIDTH, HEIGHT = ${toString width}, ${toString height}

    request = bytearray(208)
    struct.pack_into("<I", request, 0, OUTPUT)
    struct.pack_into(
        "<6I", request, 8, WIDTH, HEIGHT, Y16, 1, WIDTH * 2, WIDTH * HEIGHT * 2
    )
    device = os.open(sys.argv[1], os.O_RDWR)
    fcntl.ioctl(device, VIDIOC_S_FMT, request)
    frame = b"".join(
        struct.pack("<H", (x * 3 + y * 5) % 1024 << 6)
        for y in range(HEIGHT)
        for x in range(WIDTH)
    )
    os.write(device, frame)
    open("/run/stand-in-camera", "w").close()
    while True:
        if os.path.exists("/run/stand-in-paused"):
            time.sleep(0.05)
        else:
            os.write(device, frame)
  '';

  # Names each frame in the file it's given: the pattern in grey, blank, or
  # something else.
  frames = pkgs.writers.writePython3Bin "frames" { } ''
    import sys

    WIDTH, HEIGHT = ${toString width}, ${toString height}
    size = WIDTH * HEIGHT
    pattern = bytes(
        ((x * 3 + y * 5) % 1024) >> 2 for y in range(HEIGHT) for x in range(WIDTH)
    )
    data = open(sys.argv[1], "rb").read()
    if len(data) % size:
        sys.exit(f"{len(data)} bytes isn't a whole number of frames")
    names = []
    for start in range(0, len(data), size):
        frame = data[start:start + size]
        if frame == pattern:
            names.append("pattern")
        elif not any(frame):
            names.append("blank")
        else:
            names.append("other")
    print(" ".join(names))
  '';

  # Links the relay's source to the stand-in, found by its name, as a
  # machine's prepare program finds the node its pipeline ends at.
  prepare = pkgs.writeShellScript "find-stand-in" ''
    set -eu
    for node in /sys/class/video4linux/*; do
      if [ "$(cat "$node/name")" = "Stand-in IR" ]; then
        ln -sfn "/dev/''${node##*/}" /run/ir-camera/source
        exit 0
      fi
    done
    echo "No stand-in IR camera." >&2
    exit 1
  '';
in
pkgs.testers.runNixOSTest {
  name = "ir-camera";

  nodes.machine =
    { config, ... }:
    {
      imports = [ self.nixosModules.ir-camera ];
      jitsusama.irCamera = {
        enable = true;
        source = "/run/ir-camera/source";
        format = "Y16";
        inherit width height prepare;
      };
      environment.systemPackages = [
        camera
        frames
        config.jitsusama.v4l2loopback.package.bin
        pkgs.v4l-utils
      ];
      # The stand-in sends frames as fast as it can, so it gets a CPU of its
      # own.
      virtualisation.cores = 2;
      # udev takes its time over each camera, as it can on a busy boot, so a
      # relay let at the loopback before udev gives it to the video group
      # finds it root's alone.
      services.udev.extraRules = ''
        SUBSYSTEM=="video4linux", ACTION=="add", PROGRAM="${pkgs.coreutils}/bin/sleep 2"
      '';
    };

  testScript =
    let
      # The frames' size, which the script reads from this file at its top.
      values = pkgs.writeText "ir-camera-values.json" (builtins.toJSON { inherit width height; });
      script = builtins.readFile ./ir-camera-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
