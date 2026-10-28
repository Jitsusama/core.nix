# zsh in each account: its keys, options and completion (init.zsh and
# completion.zsh), and the terminal's marks for each command (terminal.zsh).
# The prompt is its own module (modules/prompt). It's zsh's own, with no
# framework: a shell starts no program for any of it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours;
  sgr = import ../theme/sgr.nix { inherit lib; };
  # home-manager makes the history's folder with a dirname and a mkdir at
  # every start; zsh can name the folder itself, and it's there after the
  # first.
  makesFolder = ''mkdir -p "$(dirname "$HISTFILE")"'';
  findsFolder = "[[ -d \${HISTFILE:h} ]] || mkdir -p \${HISTFILE:h}";
  rc = config.programs.zsh.initContent;
in
{
  imports = [ ../theme/home.nix ];

  programs.zsh = {
    enable = true;
    # NixOS's own zshrc asks hostname for the host at every start, which zsh
    # knows already, and sets history and aliases this account sets itself,
    # so on Linux the account's shell reads only its own files. A Mac's are
    # left as they are.
    envExtra = lib.optionalString pkgs.stdenv.hostPlatform.isLinux "setopt no_global_rcs";
    sessionVariables = {
      LANG = "en_US.UTF-8";
      LC_ALL = "en_US.UTF-8";
    };
    history = {
      extended = true;
      expireDuplicatesFirst = true;
    };
    completionInit = builtins.readFile ./completion.zsh;
    initContent = lib.mkMerge [
      # Before completion, so its styles are set when it starts, and before
      # fzf's keys, which take Ctrl+R over.
      (lib.mkOrder 560 ''
        typeset -g _zsh_selected='${sgr "48" colours.selection};${sgr "38" colours.strong}'
        ${builtins.readFile ./init.zsh}
      '')
      (builtins.readFile ./terminal.zsh)
    ];
  };

  home.file."${config.lib.zsh.dotDirRel}/.zshrc".text =
    assert lib.assertMsg (lib.hasInfix makesFolder rc)
      "home-manager no longer makes the history's folder as modules/zsh/home.nix expects";
    lib.mkForce (lib.replaceStrings [ makesFolder ] [ findsFolder ] rc);
}
