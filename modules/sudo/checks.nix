# The checks of sudo's module, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
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
// onLinux {
  # sudo typed at the terminal of an account with the module goes to
  # sudo-by-touch, which, in front of stand-ins that say what they were
  # given, hands a command or a root shell alone to run0 and every form of
  # sudo's own to sudo (by-touch-test.sh). Whether run0 then takes the
  # touch is vm-a-yubikey-answers-only-once-in's.
  a-terminal-sudo-goes-through-run0 =
    let
      saying = name: "${pkgs.writeShellScript name ''echo "${name}''${*:+ $*}"''}";
      byTouch = pkgs.callPackage ./by-touch.nix {
        run0 = saying "run0";
        sudo = saying "sudo";
      };
      alias = exampleLaptop.config.home-manager.users.joel.programs.zsh.shellAliases.sudo or "";
    in
    assert lib.assertMsg (lib.hasSuffix "/bin/sudo-by-touch" alias)
      "The example laptop's sudo is '${alias}', not sudo-by-touch.";
    pkgs.runCommandLocal "a-terminal-sudo-goes-through-run0" { } ''
      ${pkgs.bash}/bin/bash ${./by-touch-test.sh} ${lib.getExe byTouch}
      touch $out
    '';
}
