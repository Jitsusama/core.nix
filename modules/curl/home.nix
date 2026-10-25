# curl, for fetching a URL from the shell.
{ pkgs, ... }:
{
  home.packages = [ pkgs.curl ];
}
