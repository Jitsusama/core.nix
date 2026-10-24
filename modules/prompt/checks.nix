# The checks of prompt, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  pkgs,
  workstation,
  ...
}:
{
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
        export HOME=$TMPDIR/home TERM=xterm-256color
        mkdir -p $HOME/zsh repo/a/b
        echo "source ${(account workstation).jitsusama.prompt.file}" > $HOME/zsh/.zshrc
        git -C repo init -q -b main
        git init -q --ref-format=reftable -b main table
        touch repo/a/b/file
        # Typed ahead, each line a prompt; the two reads mark where to count.
        printf '%s\r' 'cd repo' ': < /dev/null' 'cd a/b' true false 'cd ../../../table' true \
          'cd ..' ': < /dev/zero' exit > keys
        ZDOTDIR=$HOME/zsh script -q -c \
          "stty cols 90 rows 30; strace -f -qq -o trace -e trace=process,openat zsh -i" \
          /dev/null < keys > /dev/null
        shell=$(head -1 trace | cut -d' ' -f1)
        awk -v shell=$shell '
          $1 == shell && /\/dev\/null/ { counting = 1 }
          $1 == shell && /\/dev\/zero/ { counting = 0 }
          counting && $1 == shell
        ' trace > prompts
        # Each program the shell ran, by name; a PATH lookup's misses fail.
        ran=$(grep 'execve("' trace | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' |
          sort -u | tr '\n' ' ')
        [ "$ran" = "git-engine mkfifo zsh " ] || { echo "The shell ran: $ran" >&2; exit 1; }
        grep -q HEAD prompts || { echo "No prompt read the repository." >&2; exit 1; }
        grep -q tables.list prompts || { echo "No prompt read the reftable." >&2; exit 1; }
        if grep -E 'clone|fork|execve' prompts; then
          echo "A prompt started a process." >&2
          exit 1
        fi
        touch $out
      '';

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
