# A Mac with a screen in front of Joel: fonts, macOS' defaults and the apps
# Homebrew installs. Ghostty and WezTerm are the Mac's terminals until kitty
# replaces them.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.base
    darwinModules.fonts
    darwinModules.macos-defaults
    darwinModules.homebrew
  ];
  home-manager.sharedModules = [
    homeModules.homebrew
    homeModules.ghostty
    homeModules.wezterm
  ];
}
