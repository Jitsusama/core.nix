{ ... }:
{
  imports = [
    ./bat
    ./btop
    ./broot
    ./claude
    ./curl
    ./dig
    ./direnv
    ./fd
    ./fzf
    ./gh
    ./ghostty
    ./git
    ./glow
    ./gnupg
    ./homebrew
    ./jq
    ./lsd
    ./neovim
    ./nixfmt
    ./opencode
    ./openssl
    ./ripgrep
    ./telnet
    ./tree
    ./tree-sitter
    ./yq
    ./zellij
    ./zoxide
    ./zsh
  ];

  home = {
    sessionPath = [
      "$HOME/.local/bin"
    ];
  };

  programs = {
    home-manager.enable = true;
  };
}
