# GnuPG, for OpenPGP keys, encryption and signatures.
{ pkgs, ... }:
{
  home.packages = [ pkgs.gnupg ];
}
