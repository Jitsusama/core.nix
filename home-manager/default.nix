{ ... }:
{
  imports = [
    ./bat
    ./btop
    ./broot
    ./claude
    ./curl
    ./dig
    ./fd
    ./fzf
    ./gh
    ./git
    ./glow
    ./gnupg
    ./homebrew
    ./jq
    ./lsd
    ./neovim
    ./nixfmt
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
