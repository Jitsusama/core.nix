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

  # A page as a person writes it in `pages`, from the repository's root.
  fromTheRoot = page: lib.removePrefix (toString ../.. + "/") (toString page);

  # What a set of mounts does that would leave a page unjudged. Each mount
  # serving pages must be a localSite built from a path in the tree, which
  # says every page it serves, so a site mounted some other way fails
  # rather than going unjudged; and each such page must be named in
  # `pages`. Home, which no mount serves, is named there by hand. A mount
  # named pageless must serve no page, so the name alone exempts nothing.
  faultsOf =
    mounts:
    let
      sited = removeAttrs mounts pageless;
      unsourced = lib.attrNames (lib.filterAttrs (_: site: !(site ? localSitePages)) sited);
      unnamed = lib.filter (page: !(lib.elem page (lib.attrValues pages))) (
        lib.concatLists (
          lib.mapAttrsToList (_: site: site.localSitePages) (
            lib.filterAttrs (_: site: site ? localSitePages) sited
          )
        )
      );
      paging = lib.attrNames (
        lib.filterAttrs (_: site: site ? localSitePages) (
          lib.getAttrs (lib.filter (name: mounts ? ${name}) pageless) mounts
        )
      );
    in
    lib.optional (
      sited == { }
    ) "a machine in a realm mounts no page, so the gate can't tell what it misses"
    ++
      lib.optional (unsourced != [ ])
        "${lib.concatStringsSep ", " unsourced} mount no localSite built from a path in the tree, so their pages can't be judged: build them so, or name them in pageless"
    ++
      lib.optional (unnamed != [ ])
        "name ${
          lib.concatMapStringsSep ", " fromTheRoot unnamed
        } in heads.nix's pages, by the name Joel knows each by"
    ++
      lib.optional (paging != [ ])
        "${lib.concatStringsSep ", " paging} serve pages, though pageless names them as serving none: take them out of pageless";

  # The faults of what the realm workstation in checks.nix mounts: the
  # sites core's own modules turn on there, under a realm and under
  # repository roots both. A mount behind an option that machine doesn't
  # turn on isn't seen, so turn its option on there too; mounts a machine's
  # own repository adds aren't seen either.
  mountFaults = faultsOf mounts;

  # The mount rules hold of a mount built each other way, so the gate is
  # seen to fail closed rather than taken to: each case but the last must
  # be refused.
  refused = {
    "no mounts" = { };
    "a plain folder" = {
      x = pkgs.emptyDirectory;
    };
    "a derivation built from a source" = {
      x = {
        src = ../mycelium/forage;
      };
    };
    "a page pages doesn't name" = {
      x = {
        localSitePages = [ ./nowhere/index.html ];
      };
    };
    "a second page pages doesn't name" = {
      x = {
        localSitePages = [
          pages.Forage
          ./nowhere/cell.html
        ];
      };
    };
    "a pageless mount serving a page" = {
      forage = {
        localSitePages = [ pages.Forage ];
      };
      local-sites = {
        localSitePages = [ pages.Map ];
      };
    };
  };
  passed = {
    forage = {
      localSitePages = [ pages.Forage ];
    };
    local-sites = pkgs.emptyDirectory;
  };
  ruleFaults =
    map (case: "the gate's own mount rules let ${case} through, so its page would go unjudged") (
      lib.attrNames (lib.filterAttrs (_: case: faultsOf case == [ ]) refused)
    )
    ++
      lib.optional (faultsOf passed != [ ])
        "the gate's own mount rules refuse a localSite named in pages: ${lib.concatStringsSep "; " (faultsOf passed)}";

  # Said in the build's log, where a change's checks show why it failed,
  # rather than at evaluation, where they'd show only that it didn't.
  faults = map (fault: "heads-are-the-kits: ${fault}") (mountFaults ++ ruleFaults);

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
pkgs.runCommandLocal "heads-are-the-kits"
  {
    env = {
      WEB_SKILL = "${../theme/web/skill}";
      PYTHONPATH = "${./tests}";
      PYTHONDONTWRITEBYTECODE = "1";
    };
  }
  ''
    ${lib.concatMapStrings (fault: "echo ${lib.escapeShellArg fault} >&2\n") faults}
    ${lib.optionalString (faults != [ ]) "exit 1"}
    ${plainPython}/bin/python -m pytest -p no:cacheprovider --import-mode=importlib \
      --basetemp="$TMPDIR/pytest" ${./tests}/heads-test.py
    ${plainPython}/bin/python ${./tests/heads.py} \
      ${lib.escapeShellArg "--waiting=${lib.concatStringsSep "," waitingForTheHead}"} ${
        lib.escapeShellArgs (lib.mapAttrsToList (name: page: "${name}=${page}") pages)
      }
    touch $out
  ''
