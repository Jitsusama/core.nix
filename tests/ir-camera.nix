# Relays a stand-in IR camera, a loopback sent 16-bit greyscale frames of a
# known pattern, to the camera programs open. What has to hold shows only with
# the devices there: the camera stays closed until a program reads, the reader
# gets the pattern in 8-bit grey, the camera closes when it stops, and the next
# reader starts on a blank frame rather than the last one's face.
{ self, pkgs }:
let
  width = 648;
  height = 368;

  # Sends the pattern to the loopback it's given, over and over, as a camera
  # would, and says when the first frame is out. The pattern is 10-bit, sent
  # as Y16, since v4l2loopback sizes Y10 frames at ten bits a pixel rather
  # than the sixteen they're stored in.
  camera = pkgs.writers.writePython3Bin "stand-in-camera" { } ''
    import fcntl
    import os
    import struct
    import sys

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
        config.boot.kernelPackages.v4l2loopback.bin
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

  testScript = ''
    machine.wait_for_unit("ir-camera.service")
    machine.wait_for_file("/dev/ir-camera")

    with subtest("the relay waits for udev to give it the loopback"):
        restarts = machine.succeed("systemctl show -P NRestarts ir-camera.service")
        t.assertEqual(restarts.strip(), "0")
        machine.fail("journalctl -b -o cat -u ir-camera.service | grep -q 'Permission denied'")

    source = machine.succeed("v4l2loopback-ctl add -n 'Stand-in IR' -x 1 -b 2").strip()
    machine.succeed(f"udevadm wait --timeout 10 {source}")
    machine.succeed(f"systemd-run --unit=stand-in -p Type=exec stand-in-camera {source}")
    machine.wait_for_file("/run/stand-in-camera")
    relay = machine.succeed("systemctl show -P MainPID ir-camera.service").strip()
    holds_camera = f"readlink /proc/{relay}/fd/* | grep -q -x -F {source}"

    with subtest("the camera stays closed until a program reads"):
        machine.fail(holds_camera)

    with subtest("programs get grey frames, and browsers don't take it for a camera"):
        format = machine.succeed("v4l2-ctl -d /dev/ir-camera --get-fmt-video")
        t.assertIn("'GREY'", format)
        t.assertIn("${toString width}/${toString height}", format)
        capabilities = machine.succeed("v4l2-ctl -d /dev/ir-camera --info")
        t.assertIn("Video Output", capabilities.split("Device Caps")[1])

    with subtest("a reader gets the camera's frames in grey"):
        machine.succeed(
            "v4l2-ctl -d /dev/ir-camera --stream-mmap --stream-count=10 --stream-to=/tmp/read"
        )
        names = machine.succeed("frames /tmp/read").split()
        print(f"The reader got: {' '.join(names)}")
        t.assertNotIn("other", names)
        t.assertEqual(names[-1], "pattern", " ".join(names))

    with subtest("the camera closes when the reader stops"):
        machine.wait_until_succeeds(f"! {holds_camera}", timeout=30)
        machine.wait_until_succeeds(
            "journalctl -b -o cat -u ir-camera.service | grep -q 'The reader stopped'", timeout=30
        )

    with subtest("the next reader starts on a blank frame, not the last one's"):
        machine.succeed("systemctl stop stand-in.service")
        machine.succeed(
            "v4l2-ctl -d /dev/ir-camera --stream-mmap --stream-count=1 --stream-to=/tmp/next"
        )
        t.assertEqual(machine.succeed("frames /tmp/next").split(), ["blank"])
  '';
}
