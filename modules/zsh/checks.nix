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
        export HOME=$TMPDIR/home TERM=xterm-kitty
        mkdir -p $HOME
        ln -s ${(account machine).home-files}/{.zshrc,.zshenv} $HOME/
        # Typed ahead, each line a prompt; the two reads mark where to count.
        touch begin end
        printf '%s\r' ": < $PWD/begin" true true true ": < $PWD/end" exit > keys
        start() {
          script -q -c "stty cols 90 rows 30; $1 zsh -i" /dev/null < keys > /dev/null
        }
        # The first start makes the completion dump the rest are read from.
        start ""
        start "strace -f -qq -o trace -e trace=process,openat"
        names() {
          grep 'execve("' | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' |
            sort -u | tr '\n' ' '
        }
        shell=$(head -1 trace | cut -d' ' -f1)
        mark() { grep -n "^$shell .*$1" trace | head -1 | cut -d: -f1; }
        started=$(head -n $(mark $PWD/begin) trace | names)
        prompts=$(sed -n "$(mark $PWD/begin),$(mark $PWD/end)p" trace | names)
        [ "$started" = "direnv git-engine mkfifo zsh " ] ||
          { echo "Starting the shell ran: $started" >&2; exit 1; }
        [ "$prompts" = "direnv " ] || { echo "Its prompts ran: $prompts" >&2; exit 1; }
        touch $out
      '';
}
