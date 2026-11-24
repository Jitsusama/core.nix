# heads-are-the-kits: the gate holding every live page to the kit's head,
# with the pages it judges and those still waiting to move to it. It's a
# file of its own because its lists change as pages move, apart from the
# other checks; checks.nix gives it what a machine in a realm mounts.
{
  lib,
  pkgs,
  mounts,
}:
let
  # The Python the gate reads markup in, with no browser.
  plainPython = pkgs.python3.withPackages (python: [ python.pytest ]);

  # Every live page's markup, by the name Joel knows it by, which the gate
  # holds to the kit's head.
  pages = {
    Code = ../repos/site/index.html;
    Forage = ../mycelium/forage/index.html;
    Home = ./home/index.html;
    Machines = ../system/site/index.html;
    Map = ../mycelium/site/index.html;
    Murmurs = ../mycelium/murmurs/index.html;
    Network = ../net/site/index.html;
  };

  # The mounts that serve no page of their own: the kit every page reads.
  pageless = [ "local-sites" ];

  # The mounts serving a page, each of which must say where its page comes
  # from, so a site mounted some other way fails here rather than going
  # unjudged.
  sited = removeAttrs mounts pageless;
  unsourced = lib.attrNames (lib.filterAttrs (_: site: !(site ? src)) sited);

  # Each page a machine in a realm mounts that `pages` doesn't name, which
  # the gate would never judge. Code, mounted only where repositories are
  # rooted, and Home, which no mount serves, are named by hand.
  unnamedPages = lib.filter (page: !(lib.elem page (lib.attrValues pages))) (
    lib.mapAttrsToList (_: site: site.src + "/index.html") (lib.filterAttrs (_: site: site ? src) sited)
  );

  # The pages still drawing a head of their own. Take each off as it moves
  # to the kit's: the gate fails while a page on it already keeps to the
  # head, and refuses a name that wasn't on it from the start, which
  # heads.py's FIRST_WAITING holds. Putting a moved page back, or a name
  # into FIRST_WAITING, is the way past the gate, which review alone holds,
  # so do it only to unblock a merge, and say so.
  waitingForTheHead = [
    "Code"
    "Forage"
    "Home"
    "Machines"
    "Map"
    "Murmurs"
    "Network"
  ];
in
assert lib.assertMsg (
  sited != { }
) "heads-are-the-kits: a machine in a realm mounts no page, so the gate can't tell what it misses";
assert lib.assertMsg (unsourced == [ ])
  "heads-are-the-kits: ${lib.concatStringsSep ", " unsourced} mount no localSite, so their pages can't be judged: build them with localSite, or name them in pageless";
assert lib.assertMsg (unnamedPages == [ ])
  "heads-are-the-kits: name ${lib.concatMapStringsSep ", " toString unnamedPages} in pages";
pkgs.runCommandLocal "heads-are-the-kits"
  {
    env = {
      WEB_SKILL = "${../theme/web/skill}";
      PYTHONPATH = "${./tests}";
      PYTHONDONTWRITEBYTECODE = "1";
    };
  }
  ''
    ${plainPython}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
      --basetemp="$TMPDIR/pytest" ${./tests}/heads-test.py
    ${plainPython}/bin/python ${./tests/heads.py} \
      ${lib.escapeShellArg "--waiting=${lib.concatStringsSep "," waitingForTheHead}"} ${
        lib.escapeShellArgs (lib.mapAttrsToList (name: page: "${name}=${page}") pages)
      }
    touch $out
  ''
