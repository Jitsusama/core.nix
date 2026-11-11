# The checks of the launcher's Passwords, which the folder's checks.nix gathers
# with every other place's; its opening comment says why each place keeps its
# own.
{
  launcherOnNiri,
  lib,
  pkgs,
  ...
}:
{
  # The Passwords place runs the passwords daemon as the shell's own child
  # and asks it over its standard input and output, its one manager a
  # stand-in for 1Password's that never runs op: nothing until
  # Joel steps in, unlocking it from its first row, an entry's Return
  # copying its password through wl-copy as the launcher goes, the margin
  # wearing the clipboard's mark and the foot, opened again, counting to
  # its clearing, resting on one reading it in full, stepping into one
  # holding its acts in order, and Open the Website handing a stand-in
  # Chrome its website as the launcher goes
  # (tests/passwords-on-niri-test.py). A VM adds only 1Password's own
  # manager, which passwords-runs-each-password-manager reads.
  the-passwords-place-copies-through-its-daemon-on-niri = launcherOnNiri {
    name = "the-passwords-place-copies-through-its-daemon-on-niri";
    tests = ../../../tests/passwords-on-niri-test.py;
    # On here, where no agent runs and no secret is real, copying through
    # wl-copy, its one manager the stand-in, which logs where the test's
    # environment says.
    shell.launcher.providers.passwords = {
      off = lib.mkForce false;
      copier = "wl-copy";
      daemon = lib.mkForce [
        (lib.getExe (pkgs.callPackage ../../passwords/package.nix { }))
        "daemon"
        "${pkgs.writeText "password-managers.json" (
          builtins.toJSON {
            managers = [
              {
                name = "1password";
                command = [
                  (lib.getExe pkgs.bash)
                  "${../../../tests/passwords-stand-in.sh}"
                ];
              }
            ];
          }
        )}"
        "${(import ../launcher/words.nix).file}"
      ];
    };
  };

  # Off, as the module leaves it until agents are fenced off, Home's
  # Passwords row and the place's one row each have a drawer saying why
  # and what Return opens meanwhile (tests/passwords-off-on-niri-test.py).
  the-passwords-place-says-why-its-off-on-niri = launcherOnNiri {
    name = "the-passwords-place-says-why-its-off-on-niri";
    tests = ../../../tests/passwords-off-on-niri-test.py;
  };

  # Off, with no password manager set up, the place's one row can open
  # nothing, so its Return goes into its drawer, which says why
  # (tests/passwords-off-into-its-drawer-on-niri-test.py).
  the-passwords-place-off-goes-into-its-drawer-on-niri = launcherOnNiri {
    name = "the-passwords-place-off-goes-into-its-drawer-on-niri";
    tests = ../../../tests/passwords-off-into-its-drawer-on-niri-test.py;
    shell.launcher.providers.passwords.managers = lib.mkForce [ ];
  };
}
