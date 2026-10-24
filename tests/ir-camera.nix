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

  # The frames' size, which the script and the stand-ins read from this
  # file.
  values = pkgs.writeText "ir-camera-values.json" (builtins.toJSON { inherit width height; });
  # A Python file of ir-camera/, given the values, and held to ruff and mypy
  # through `nix fmt` rather than the writers' flake8.
  program =
    name:
    pkgs.writers.writePython3Bin name { doCheck = false; } (
      builtins.replaceStrings [ "@values@" ] [ "${values}" ] (builtins.readFile ./ir-camera/${name}.py)
    );
  # Sends the pattern to a loopback over and over, as the camera would
  # (ir-camera/stand-in-camera.py).
  camera = program "stand-in-camera";
  # Names each frame in a file: the pattern in grey, blank, or something
  # else (ir-camera/frames.py).
  frames = program "frames";

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

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./ir-camera-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
