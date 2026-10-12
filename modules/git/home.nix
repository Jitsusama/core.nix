{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama) identity;
  declared = "${config.xdg.configHome}/git/declared.config";
  declaredLast = pkgs.writeShellApplication {
    name = "declared-last";
    runtimeInputs = [
      config.programs.git.package
      pkgs.coreutils
      pkgs.gnused
    ];
    text = builtins.readFile ./declared-last.sh;
  };
in
{
  imports = [ ../identity/home.nix ];

  options.jitsusama.git.programIncludes = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    description = ''
      Files programs keep their own git settings in and include from
      ~/.gitconfig themselves. Each switch includes them there ahead of the
      declared settings, where the program looks, so it finds its include
      and adds no other below them.
    '';
  };

  config = {
    programs.zsh.oh-my-zsh.plugins = [ "git" ];

    # Programs change their own settings with git config --global, so the file
    # git writes, ~/.gitconfig, stays a plain file, and it includes what's
    # declared here at its end, where git reads it over what they wrote. Nothing
    # is linked at ~/.config/git/config, so an older generation's link there has
    # nothing in its way.
    xdg.configFile."git/config".target = "git/declared.config";
    home.activation.gitReadsTheDeclaredLast = lib.hm.dag.entryAfter [ "linkGeneration" ] ''
      run ${lib.getExe declaredLast} ${
        lib.escapeShellArgs (
          [
            declared
            "${config.home.homeDirectory}/.gitconfig"
          ]
          ++ config.jitsusama.git.programIncludes
        )
      }
    '';

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
  };
}
