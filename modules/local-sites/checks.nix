# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{ lib, pkgs, ... }:
{
  # The mycelium site's writer keeps its last map while no node answers, and
  # writes the live sessions alone, without a message's words, once one does
  # (tests/mycelium-map-keeps-its-last-map.sh).
  mycelium-map-keeps-its-last-map = pkgs.runCommandLocal "mycelium-map-keeps-its-last-map" { } ''
    ${lib.getExe pkgs.bash} ${./tests/mycelium-map-keeps-its-last-map.sh} ${
      lib.escapeShellArgs [
        ./mycelium-map.sh
        ./mycelium-map.jq
        (lib.getExe pkgs.jq)
        "${pkgs.coreutils}/bin"
      ]
    }
    touch $out
  '';
}
