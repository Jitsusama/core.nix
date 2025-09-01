{
  ...
}:
{
  programs.zsh.oh-my-zsh.plugins = [ "git" ];

  programs.git = {
    enable = true;
    userName = "Joel Gerber";
    aliases = {
      prune-branches = "!git fetch --prune && git branch -vv | grep ': gone]' | awk '{print $1}' | xargs -r git branch -D";
    };
    extraConfig = {
      core = {
        autocrlf = "input";
        editor = "code -w";
      };
      init.defaultBranch = "main";
      pull.rebase = true;
      push.autoSetupRemote = true;
      diff.tool = "vscode";
      difftool.vscode.cmd = "code --wait --diff $LOCAL $REMOTE";
      merge.tool = "vscode";
      mergetool.vscode.cmd = "code --wait $MERGED";
      filter.lfs = {
        clean = "git-lfs clean -- %f";
        smudge = "git-lfs smudge -- %f";
        process = "git-lfs filter-process";
        required = true;
      };
    };
    ignores = [
      ".tool-version"
      ".vscode/"
      "*.code-workspace"
      "*.idea/"
      "*.iml"
      "*.DS_Store"
      ".bundle/config"
    ];
  };
}
