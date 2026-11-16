# The checks of local-sites, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{ lib, pkgs, ... }:
{
  # The home's builder lists each site from its row, settled ones apart,
  # strips each page that asks, and changes nothing on a second run
  # (tests/home-builds-from-its-rows.sh).
  home-builds-from-its-rows = pkgs.runCommandLocal "home-builds-from-its-rows" { } ''
    ${lib.getExe pkgs.bash} ${./tests/home-builds-from-its-rows.sh} ${
      lib.escapeShellArgs [
        (lib.getExe pkgs.python3)
        ./home
      ]
    }
    touch $out
  '';

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
