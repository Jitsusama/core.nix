# Says which programs have a loopback camera open. What has to hold shows
# only with a real loopback and programs of another account holding it: the
# file names each one, by its program's own name even when Nix wraps it,
# leaves out the relay that fills the camera by its unit, however a program
# names itself, keeps each name to one line, catches a holder that changes
# without an open or close, empties when they close it, stays in a folder
# that outlives the camera's service and says nobody while it's down,
# follows the camera to its next loopback, takes no loopback of another
# name for it, and waits rather than failing while the camera isn't there.
# The tests are loopback-holders-vm-test.py, which pytest runs inside the
# driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # Holds the loopback it's given open as a relay would, writing nothing;
  # and takes the name it's given, any bytes, then holds the camera open.
  # ruff and mypy read them through `nix fmt`, so the writers' flake8 is off.
  helper =
    name:
    pkgs.writers.writePython3Bin name { doCheck = false; } (
      builtins.readFile ./loopback-holders/${name}.py
    );
  relay = helper "stand-in-relay";
  renamed = helper "renamed";

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

  # What the tests need from Nix, their `values` fixture: the file naming the
  # stand-in's holders, and the programs that hold it, none of them on the
  # machine's PATH.
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

  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./loopback-holders-vm-test.py}", values="${values}")
  '';
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of these lines would add only
  # its time.
  skipTypeCheck = true;
}
