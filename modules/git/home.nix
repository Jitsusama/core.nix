{ config, lib, ... }:
let
  inherit (config.jitsusama) identity;
in
{
  imports = [ ../identity/home.nix ];

  programs.zsh.oh-my-zsh.plugins = [ "git" ];

  programs.git = {
    enable = true;
    settings = {
      user = {
        inherit (identity) name;
        email = lib.mkIf (identity.email != null) identity.email;
      };
      # Deletes the local branches whose upstream has gone, such as merged ones.
      aliases.prune-branches = lib.concatStringsSep " | " [
        "!git fetch --prune && git branch -vv"
        "grep ': gone]'"
        "awk '{print $1}'"
        "xargs -r git branch -D"
      ];
      core.autocrlf = "input";
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
      "**/.bundle/config"
      ".review/"
      ".worktrees/"
      "**/.pi/*"
      "!**/.pi/extensions/"
      "!**/.pi/prompts/"
      "!**/.pi/skills/"
      "!**/.pi/themes/"
    ];
  };
}
