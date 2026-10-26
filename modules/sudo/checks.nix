# The checks of sudo's module, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  holds,
  lib,
  macWorkstation,
  ...
}:
{
  # Every Mac lets Touch ID answer sudo, before the password and in the
  # file macOS updates keep.
  a-mac-sudo-takes-touch-id =
    let
      sudoLocal = macWorkstation.config.environment.etc."pam.d/sudo_local";
    in
    holds "a-mac-sudo-takes-touch-id"
      "A Mac's sudo doesn't let Touch ID answer it, so every switch at the Mac asks for the password."
      (sudoLocal.enable && lib.hasInfix "auth       sufficient     pam_tid.so" sudoLocal.text);
}
