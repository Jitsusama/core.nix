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
  # the way of a rollback.
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
      homeDir = home.home.homeDirectory;
      declared = home.programs.git.settings.user.name;
    in
    pkgs.runCommandLocal "git-settings-stay-writable" { nativeBuildInputs = [ pkgs.git ]; } ''
      export HOME=$PWD/home GIT_CONFIG_NOSYSTEM=1
      mkdir -p home/.config/git
      for linked in .gitconfig .config/git/config; do
        if [ -e ${home.home-files}/$linked ]; then
          echo "~/$linked is a link to the store"
          exit 1
        fi
      done
      cp ${home.home-files}/.config/git/declared.config home/.config/git/
      sed -n '\|declared-last|{s|^run ||;s|${lib.escapeRegex homeDir}|'"$HOME"'|g;p}' \
        ${home.home.activationPackage}/activate > step
      expect() {
        if [ "$(git config --get "$1")" != "$2" ]; then
          echo "$3: $1 is '$(git config --get "$1")', not '$2'"
          exit 1
        fi
      }
      mkdir -p home/.config/program
      printf '[user]\n\tname = program\n[program]\n\tsetting = 1\n' > home/.config/program/gitconfig
      includes() {
        git config --global --no-includes --get-all include.path | tr '\n' ' '
      }
      # The count the program makes before it adds its include, as dev does.
      programs() {
        git config --global --no-includes --get-all include.path |
          sed "s|^~/|$HOME/|" | grep -cxF "$HOME/.config/program/gitconfig"
      }
      sh -e step
      if [ "$(includes)" != "$HOME/.config/program/gitconfig $HOME/.config/git/declared.config " ]; then
        echo "after a switch: ~/.gitconfig includes $(includes)"
        exit 1
      fi
      expect user.name ${lib.escapeShellArg declared} "after a switch"
      expect program.setting 1 "after a switch"
      git config --global --fixed-value --unset-all include.path "$HOME/.config/program/gitconfig"
      git config --global --add include.path "~/.config/program/gitconfig"
      sh -e step
      if [ "$(programs)" != 1 ]; then
        echo "after a switch: the program's include is in ~/.gitconfig $(programs) times"
        exit 1
      fi
      printf '[user]\n\tname = included\n' > home/tool.config
      git config --global --add include.path "$HOME/tool.config"
      git config --global tool.setting 1
      git config --global user.name tool
      expect tool.setting 1 "after a program wrote it"
      sh -e step
      expect user.name ${lib.escapeShellArg declared} "after the next switch"
      expect tool.setting 1 "after the next switch"
      cp home/.gitconfig before
      sh -e step
      if ! cmp -s before home/.gitconfig; then
        echo "a switch with nothing to do changed ~/.gitconfig"
        exit 1
      fi
      touch $out
    '';
}
