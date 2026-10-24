# The checks of prompt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
# What the prompt starts is traced with strace, which only Linux has, on the
# NixOS workstation's own prompt.
onLinux {
  # The prompt starts nothing once the shell is up: across a run of prompts
  # in and out of a repository, with git-engine answering, the shell itself
  # forks and executes nothing, and its reads of HEAD show the prompts ran.
  # Starting up, it runs only git-engine and the mkfifo it talks through.
  the-prompt-starts-nothing =
    pkgs.runCommandLocal "the-prompt-starts-nothing"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.strace
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/the-prompt-starts-nothing.sh} ${(account workstation).jitsusama.prompt.file}
        touch $out
      '';
}
// {
  # The prompt gives way as tests/prompt.zsh says, without gitstatusd, which
  # needs job control a shell run this way hasn't got, in a repository that
  # keeps its refs in files and in one that keeps them in reftable, as World
  # does.
  the-prompt-gives-way =
    pkgs.runCommandLocal "the-prompt-gives-way"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home
        mkdir -p $HOME repo/modules/btop
        grep -v '^typeset -g _prompt_gitstatus=' ${(account workstation).jitsusama.prompt.file} > prompt.zsh
        for refs in files reftable; do
          rm -rf repo
          mkdir -p repo/modules/btop
          git -C repo init -q --ref-format=$refs -b jitsusama/prompt-collapse
          mkdir repo/.git/rebase-merge
          echo 2 > repo/.git/rebase-merge/msgnum
          echo 5 > repo/.git/rebase-merge/end
          zsh -f -i -c \
            "source ${../../tests/prompt.zsh} $PWD/prompt.zsh $PWD/repo/modules/btop jitsusama/prompt-collapse"
        done
        touch $out
      '';
}
