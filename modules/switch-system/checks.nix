# The checks of switch-system, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # A system the profile holds is switched to unasked, unless its PAM let a
  # YubiKey's touch in alone (switch-system-known.sh): in any service but
  # polkit's, or in polkit's without the check that the person is in
  # (known-test.sh).
  an-unasked-switch-never-brings-back-a-touch-alone =
    pkgs.runCommandLocal "an-unasked-switch-never-brings-back-a-touch-alone"
      {
        nativeBuildInputs = [
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.systemd
        ];
      }
      ''
        bash ${./known-test.sh} ${./switch-system-known.sh}
        touch $out
      '';
}
