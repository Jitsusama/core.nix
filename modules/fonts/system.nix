# The faces the theme draws with, for every account on the machine, on NixOS
# and a Mac alike.
{ pkgs, ... }:
let
  # The serif face as its two variable files, upright and italic, with every
  # axis: the package's static cuts carry the same family name at each
  # optical size and softness, so fontconfig could answer with one of those
  # rather than the file the theme's axes are set on.
  serif = pkgs.runCommand "fraunces-variable" { } ''
    mkdir -p $out/share/fonts/truetype
    ln -s ${pkgs.fraunces}/share/fonts/truetype/*'[SOFT,WONK,opsz,wght].ttf' $out/share/fonts/truetype/
  '';
  # The script face's Hebrew and Arabic alone, rather than every script
  # Noto draws, which would be a gigabyte of fonts nothing names.
  scripts = pkgs.noto-fonts.override {
    variants = [
      "Noto Sans Hebrew"
      "Noto Naskh Arabic"
    ];
  };
in
{
  fonts.packages = [
    pkgs.mona-sans
    pkgs.nerd-fonts.fira-code
    pkgs.nerd-fonts.monaspace
    pkgs.nerd-fonts.victor-mono
    pkgs.noto-fonts-cjk-sans
    scripts
    serif
  ];
}
