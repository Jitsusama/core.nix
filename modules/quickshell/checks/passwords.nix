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
  # The Passwords place asks the passwords daemon over its socket, its one
  # manager a stand-in for 1Password's that never runs op: nothing until
  # Joel steps in, unlocking it from its first row, an entry's Return
  # copying its password through wl-copy as the launcher goes, the margin
  # wearing the clipboard's mark and the foot, opened again, counting to
  # its clearing, resting on one reading it in full, stepping into one
  # holding its acts in order, and Open the Website handing a stand-in
  # Chrome its website as the launcher goes
  # (tests/passwords-on-niri-test.py). A VM adds only the units, which
  # passwords-runs-each-password-manager reads.
  the-passwords-place-copies-through-its-daemon-on-niri = launcherOnNiri {
    name = "the-passwords-place-copies-through-its-daemon-on-niri";
    tests = ../../../tests/passwords-on-niri-test.py;
    given = [
      "PASSWORDS=${lib.getExe (pkgs.callPackage ../../passwords/package.nix { })}"
      "PASSWORDS_STAND_IN=${../../../tests/passwords-stand-in.sh}"
    ];
    # On here, where no agent runs and no secret is real.
    shell.launcher.providers.passwords.off = lib.mkForce false;
  };

  # Off, as the module leaves it until agents are fenced off, Home's
  # Passwords row and the place's one row each have a drawer saying why
  # and what Return opens meanwhile (tests/passwords-off-on-niri-test.py).
  the-passwords-place-says-why-its-off-on-niri = launcherOnNiri {
    name = "the-passwords-place-says-why-its-off-on-niri";
    tests = ../../../tests/passwords-off-on-niri-test.py;
  };
}
