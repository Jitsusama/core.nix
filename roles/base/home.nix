# Every machine, a server included: zsh and its prompt, git and the
# command-line tools used daily.
{ homeModules }:
{
  imports = [
    homeModules.home-manager
    homeModules.zsh
    homeModules.prompt
    homeModules.git
    homeModules.bat
    homeModules.delta
    homeModules.broot
    homeModules.direnv
    homeModules.fzf
    homeModules.lsd
    homeModules.zoxide
    homeModules.btop
    homeModules.curl
    homeModules.dig
    homeModules.fd
    homeModules.gh
    homeModules.glow
    homeModules.gnupg
    homeModules.jq
    homeModules.nixfmt
    homeModules.openssl
    homeModules.ripgrep
    homeModules.telnet
    homeModules.tree
    homeModules.tree-sitter
    homeModules.vivid
    homeModules.yq
  ];
}
