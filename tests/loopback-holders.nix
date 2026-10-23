# Says which programs have a loopback camera open. What has to hold shows
# only with a real loopback and programs of another account holding it: the
# file names each one, by its program's own name even when Nix wraps it,
# leaves out the relay that fills the camera by its unit, however a program
# names itself, keeps each name to one line, catches a holder that changes
# without an open or close, empties when they close it, stays in a folder
# that outlives the camera's service and says nobody while it's down,
# follows the camera to its next loopback, takes no loopback of another
# name for it, and waits rather than failing while the camera isn't there.
# The script is loopback-holders-vm-test.py.
{ self, pkgs }:
let
  # Holds the loopback it's given open as a relay would, writing nothing.
  relay = pkgs.writers.writePython3Bin "stand-in-relay" { } ''
    import os
    import sys
    import time

    device = os.open(sys.argv[1], os.O_RDWR)
    while True:
        time.sleep(60)
  '';

  # Takes the name it's given, any bytes, then holds the camera open.
  renamed = pkgs.writers.writePython3Bin "renamed" { } ''
    import ctypes
    import os
    import sys
    import time

    PR_SET_NAME = 15
    name = sys.argv[1].encode().decode("unicode_escape").encode("latin-1")
    ctypes.CDLL(None).prctl(PR_SET_NAME, ctypes.c_char_p(name), 0, 0, 0)
    device = os.open(sys.argv[2], os.O_RDONLY)
    while True:
        time.sleep(60)
  '';

  # A program as Nix wraps one, whose process is .doze-wrapped: it waits, so
  # it holds whatever it's handed open until it's stopped.
  doze =
    pkgs.runCommand "doze"
      {
        nativeBuildInputs = [ pkgs.makeWrapper ];
        waits = pkgs.writeCBin "doze" "#include <unistd.h>\nint main(void) { for (;;) pause(); }\n";
      }
      ''
        mkdir -p $out/bin
        cp $waits/bin/doze $out/bin/doze
        wrapProgram $out/bin/doze
      '';

  # What the script needs from Nix, which it reads from this file at its top:
  # the file naming the stand-in's holders, and the programs that hold it,
  # none of them on the machine's PATH.
  values = pkgs.writeText "loopback-holders-values.json" (
    builtins.toJSON {
      file = "/run/camera-holders/stand-in";
      bash = "${pkgs.bash}/bin/bash";
      sleep = "${pkgs.coreutils}/bin/sleep";
      doze = "${doze}/bin/doze";
      renamed = pkgs.lib.getExe renamed;
    }
  );
in
pkgs.testers.runNixOSTest {
  name = "loopback-holders";

  nodes.machine =
    { config, ... }:
    let
      loopback = "${config.jitsusama.v4l2loopback.package.bin}/bin/v4l2loopback-ctl";
    in
    {
      imports = [ "${self}/modules/v4l2loopback/nixos.nix" ];
      jitsusama.v4l2loopback = {
        enable = true;
        holders.stand-in = {
          device = "/run/stand-in/device";
          unit = "stand-in.service";
          card = "Stand-in";
          ignoreUnits = [ "stand-in.service" ];
        };
      };
      systemd.services.stand-in = {
        wantedBy = [ "multi-user.target" ];
        # As the camera's relay does, since nothing else loads the module.
        wants = [ "modprobe@v4l2loopback.service" ];
        after = [ "modprobe@v4l2loopback.service" ];
        serviceConfig.RuntimeDirectory = "stand-in";
        preStart = "${loopback} add -n 'Stand-in' > /run/stand-in/device";
        script = ''exec ${pkgs.lib.getExe relay} "$(cat /run/stand-in/device)"'';
        postStop = ''${loopback} delete "$(cat /run/stand-in/device)" || true'';
      };
      users.users.someone = {
        isNormalUser = true;
        extraGroups = [ "video" ];
      };
      environment.systemPackages = [
        pkgs.v4l-utils
        config.jitsusama.v4l2loopback.package.bin
      ];
    };

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./loopback-holders-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
