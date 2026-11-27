# The checks of the web-design skill's probes, which checks.nix gathers with
# every other folder's: each probe run in headless Chromium over small pages
# of its own, by a test beside it here. Chromium builds only on Linux.
{
  lib,
  onLinux,
  pkgs,
  ...
}:
let
  # The browser the probe tests share, a pytest plugin put on Python's path
  # alone, so an edit beside it rebuilds neither.
  probeBrowser = lib.fileset.toSource {
    root = ./.;
    fileset = ./probe_browser.py;
  };
  python = pkgs.python3.withPackages (python: [
    python.playwright
    python.pytest
  ]);
  # A check named `name` running `test` with the skill's probes beside it,
  # and `env` added to what every probe test is given.
  probed =
    name: test: env:
    pkgs.runCommandLocal name
      {
        env = {
          WEB_SKILL = "${./skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PYTHONPATH = "${probeBrowser}";
          PYTHONDONTWRITEBYTECODE = "1";
        }
        // env;
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider -p probe_browser \
          --basetemp="$TMPDIR/pytest" ${test}
        touch $out
      '';
  # Without a fontconfig, the sandbox's Chromium fails a web face it fetched
  # as often as it draws it.
  noFonts = pkgs.makeFontsConf { fontDirectories = [ ]; };
in
onLinux {
  # The first probe's masthead (gate-test.py): a mark in the head or the
  # one element just before it, as the live sites' head draws it, passes,
  # and one further up, only the switch's icons, or no head at all is asked
  # for, each rule pinned from both sides.
  gate-probe-finds-the-sites-mark = probed "gate-probe-finds-the-sites-mark" ./gate-test.py { };
  # The homes probe (homes-test.py): a pattern's names learned from the
  # page's links on the sites alone, each row and record the system reader
  # drops named, a kind's first claim kept, and a shortened name's tail read
  # as no mention.
  homes-probe-reads-as-the-reader = probed "homes-probe-reads-as-the-reader" ./homes-test.py { };
  # The looks probe (looks-test.py): what a page draws beyond its words,
  # measured before a deploy and after it, with the deploy of 18 November
  # 2026 that kept a guide's words and lost its masthead, art and
  # stylesheets as its first case.
  looks-probe-finds-what-a-deploy-lost =
    probed "looks-probe-finds-what-a-deploy-lost" ./looks-test.py
      {
        FACE = "${pkgs.fraunces}/share/fonts/truetype/Fraunces[SOFT,WONK,opsz,wght].ttf";
        FONTCONFIG_FILE = noFonts;
      };
  # The pixels probe (pixels-test.py), over small pages at a phone's width:
  # a page wider than its window, a part arriving that shifts the words,
  # text under 12 px and a drawing's label under 9 px, faint text, and a
  # control named apart from what it shows, each from what the live sites
  # did on 22 November 2026.
  pixels-probe-holds-every-pixel = probed "pixels-probe-holds-every-pixel" ./pixels-test.py {
    FONTCONFIG_FILE = noFonts;
  };
  # The drawing words probe (svgtext-test.py): words inside an inline SVG
  # measured against the shapes and page behind them, with the forage
  # research page's diagram of 15 November 2026, light words on a block
  # that fell back to paper in the dark look, as its first case.
  svgtext-probe-measures-a-drawings-words =
    probed "svgtext-probe-measures-a-drawings-words" ./svgtext-test.py
      {
        FONTCONFIG_FILE = noFonts;
      };
}
