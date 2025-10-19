{
  ...
}:
{
  programs.zsh.oh-my-zsh.plugins = [ "git" ];

  programs.git = {
    enable = true;
    settings = {
      user = {
        name = "Joel Gerber";
      };
      aliases = {
        prune-branches = "!git fetch --prune && git branch -vv | grep ': gone]' | awk '{print $1}' | xargs -r git branch -D";
      };
      core = {
        autocrlf = "input";
      };
      filter.lfs = {
        clean = "git-lfs clean -- %f";
        smudge = "git-lfs smudge -- %f";
        process = "git-lfs filter-process";
        required = true;
      };
      init.defaultBranch = "main";
      pull.rebase = true;
      push.autoSetupRemote = true;
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
