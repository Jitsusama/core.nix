# mycelium-map-check (modules/mycelium/site/map-check.sh): the map's
# live-page test, pointed at the page this machine serves. The home gives
# it the Python with Playwright and pytest, the test and the folder its
# plugin is in; the check of how soon it answers
# (mycelium-map-check-answers-soon) gives it a slow stand-in for all three.
{
  lib,
  pkgs,
  python ? lib.getExe (
    pkgs.python3.withPackages (python: [
      python.playwright
      python.pytest
    ])
  ),
  test ? "${../mycelium/site/tests/live-reach-test.py}",
  livePage ? "${./tests}",
}:
pkgs.writeShellApplication {
  name = "mycelium-map-check";
  runtimeInputs = [ pkgs.jq ];
  # It carries only the headless shell it launches.
  runtimeEnv.PLAYWRIGHT_BROWSERS_PATH = "${pkgs.callPackage ./headless-browsers.nix { }}";
  text =
    builtins.replaceStrings
      [ "@python@" "@test@" "@live_page@" ]
      [ python test livePage ]
      (builtins.readFile ../mycelium/site/map-check.sh);
}
