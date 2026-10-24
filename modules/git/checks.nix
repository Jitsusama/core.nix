# The checks of git, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
onLinux {
  # Programs change their own git settings with git config --global, which
  # fails on a file the store holds, and what's declared still has to win
  # over what they wrote. So the check runs the switch's own step, with the
  # account's home moved here, then writes as programs do, an include of
  # their own file among them, and switches again. A program that includes
  # its file itself has to find it ahead of the declared settings after the
  # first switch, once, or it adds another below them, as dev does. Neither
  # global file may be a link, or an older generation's link would be in
  # the way of a rollback. tests/git-settings-stay-writable.sh runs it.
  git-settings-stay-writable =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone =
            { config, ... }:
            {
              jitsusama.git.programIncludes = [ "${config.xdg.configHome}/program/gitconfig" ];
            };
        }
      ]);
    in
    pkgs.runCommandLocal "git-settings-stay-writable" { nativeBuildInputs = [ pkgs.git ]; } ''
      ${lib.getExe pkgs.bash} ${../../tests/git-settings-stay-writable.sh} \
        ${home.home-files} ${home.home.activationPackage}/activate \
        ${lib.escapeShellArgs [
          home.home.homeDirectory
          home.programs.git.settings.user.name
        ]}
      touch $out
    '';
}
