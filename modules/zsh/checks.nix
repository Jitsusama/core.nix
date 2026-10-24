# The checks of zsh, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
{
  # home-manager configures zsh in every account but leaves the login shell to
  # the system, so an account on NixOS has to be given zsh to read any of it.
  # A Mac's accounts log in to zsh already.
  zsh-is-the-login-shell =
    holds "zsh-is-the-login-shell" "An account on NixOS doesn't log in to zsh."
      (lib.getName (bareNixos [ self.nixosModules.base ]).config.users.users.someone.shell == "zsh");

  # The account's whole shell, not only its prompt: once its completion dump
  # is made, starting it runs git-engine, its fifo and direnv's first look,
  # and each prompt runs direnv alone, asking it whether the folder's .envrc
  # changed. Anything else a shell started would show here. Its history is
  # kept in the sandbox's home, since the account's own isn't there to hold
  # it.
  the-shell-starts-nothing =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        { home-manager.users.someone.programs.zsh.history.path = "$HOME/.zsh_history"; }
      ];
    in
    pkgs.runCommandLocal "the-shell-starts-nothing"
      {
        nativeBuildInputs = [
          pkgs.strace
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/the-shell-starts-nothing.sh} ${(account machine).home-files}
        touch $out
      '';
}
