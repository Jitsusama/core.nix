# Holds and taps dictation's chord on a stand-in keyboard, and listens as an
# account outside the input group, which can't read the keyboard itself.
# dictation-chord takes the keyboard as it appears and tells that account
# how long a held chord and a tap were held, and tells nothing of D alone,
# of another key or of Super pressed after D. The script is
# dictation-chord-vm-test.py.
{ self, pkgs }:
let
  # Presses keys on a stand-in keyboard. Each argument is a key and a
  # direction, such as leftmeta+ or d-, or a pause in ms, such as 600. The
  # keyboard waits a second first, so dictation-chord has taken it.
  keys =
    pkgs.writers.writePython3Bin "chord-keys"
      {
        libraries = [ pkgs.python3Packages.evdev ];
      }
      ''
        import sys
        import time

        from evdev import UInput
        from evdev import ecodes as e

        KEYS = {"d": e.KEY_D, "a": e.KEY_A, "leftmeta": e.KEY_LEFTMETA}

        with UInput({e.EV_KEY: list(KEYS.values())}, name="stand-in keyboard") as kb:
            time.sleep(1)
            for step in sys.argv[1:]:
                if step.isdigit():
                    time.sleep(int(step) / 1000)
                    continue
                kb.write(e.EV_KEY, KEYS[step[:-1]], 1 if step[-1] == "+" else 0)
                kb.syn()
            time.sleep(0.5)
      '';
in
pkgs.testers.runNixOSTest {
  name = "dictation-chord";

  nodes.machine = {
    imports = [ self.nixosModules.dictation ];
    boot.kernelModules = [ "uinput" ];
    users.users.alice.isNormalUser = true;
    environment.systemPackages = [
      keys
      pkgs.socat
    ];
  };

  testScript = builtins.readFile ./dictation-chord-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
