# The serif face upright at large, cut into a font of its own by
# serif-text.py, named font.serif.text.family: for a program that sets words
# for reading and can't set a variable font's axes. It takes the theme's
# font, so each account's cut is at its own theme's axes. Its file is
# `file`, for a program given a font by its path.
{ pkgs, font }:
let
  inherit (pkgs) lib;
  axes = font.serif.axes // font.serif.upright.large;
  python = pkgs.python3.withPackages (packages: [ packages.fonttools ]);
  name = "${lib.replaceStrings [ " " ] [ "" ] font.serif.text.family}.ttf";
  cut = pkgs.runCommandLocal "serif-text" { } ''
    mkdir -p $out/share/fonts/truetype
    ${python}/bin/python3 ${./serif-text.py} \
      ${pkgs.fraunces}/share/fonts/truetype/'Fraunces[SOFT,WONK,opsz,wght].ttf' \
      ${lib.escapeShellArg font.serif.text.family} $out/share/fonts/truetype/${name} \
      ${lib.concatStringsSep " " (lib.mapAttrsToList (tag: value: "${tag}=${toString value}") axes)}
  '';
in
cut // { file = "${cut}/share/fonts/truetype/${name}"; }
