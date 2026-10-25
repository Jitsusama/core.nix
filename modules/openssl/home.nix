# openssl's command line, for reading and making certificates and keys.
{ pkgs, ... }:
{
  home.packages = [ pkgs.openssl ];
}
