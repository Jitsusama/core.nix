# yq, jq for YAML, TOML and XML.
{ pkgs, ... }:
{
  home.packages = [ pkgs.yq ];
}
