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
  # The Python the gate reads markup in, with no browser, and the parser
  # that builds a page's tree by HTML's own rules.
  plainPython = pkgs.python3.withPackages (python: [
    python.pytest
    python.selectolax
  ]);
  sites = import ./site.nix { inherit lib pkgs; };

  # Every live page's markup, by the name Joel knows it by, which the gate
  # holds to the kit's head. Home's is the one home.nix builds the home from.
  pages = {
    Code = ../repos/site/index.html;
    Forage = ../mycelium/forage/index.html;
    Home = import ./home-source.nix + "/index.html";
    Machines = ../system/site/index.html;
    Map = ../mycelium/site/index.html;
    Murmurs = ../mycelium/murmurs/index.html;
    Network = ../net/site/index.html;
  };

  # A page as a person writes it in `pages`, from the repository's root.
  fromTheRoot = page: lib.removePrefix (toString ../.. + "/") (toString page);

  # What a set of mounts does that would leave a page unjudged, against the
  # pages `named`. Each mount must be a localSite built from a path in the
  # tree, which says every page it serves, so a site mounted some other way
  # fails rather than going unjudged, and each such page must be named. A
  # localSite serves its index.html at least, so one naming no page fails
  # too, but for what the sites share (site.nix's shared), which says it
  # serves none. Home, which no mount serves, is named in `pages` from the
  # folder home.nix builds it from. No name exempts a mount. What a mount
  # says of itself is trusted: one may write site.nix's passthru by hand,
  # which only review catches. It can't be known by rebuilding it, since
  # the workstation's mounts are built for another system than the one the
  # checks may evaluate on.
  isShared = site: site.sharedBySites or false;
  faultsOf =
    named: mounts:
    let
      sourced = lib.filterAttrs (_: site: site ? localSitePages) mounts;
      unsourced = lib.attrNames (removeAttrs mounts (lib.attrNames sourced));
      unpaged = lib.attrNames (
        lib.filterAttrs (_: site: site.localSitePages == [ ] && !(isShared site)) sourced
      );
      served = lib.concatLists (lib.mapAttrsToList (_: site: site.localSitePages) sourced);
      unnamed = lib.filter (page: !(lib.elem page (lib.attrValues named))) served;
    in
    lib.optional (
      served == [ ] && unsourced == [ ] && unpaged == [ ]
    ) "a machine in a realm mounts no page, so the gate can't tell what it misses"
    ++
      lib.optional (unsourced != [ ])
        "${lib.concatStringsSep ", " unsourced} mount no localSite built from a path in the tree, so their pages can't be judged: build them with localSite from the site's folder"
    ++
      lib.optional (unpaged != [ ])
        "${lib.concatStringsSep ", " unpaged} name no page, though a localSite serves its index.html: build them from a folder holding one"
    ++
      lib.optional (unnamed != [ ])
        "name ${
          lib.concatMapStringsSep ", " fromTheRoot unnamed
        } in heads.nix's pages, by the name Joel knows each by";

  # The faults of what the realm workstation in checks.nix mounts: the
  # sites core's own modules turn on there, under a realm and under
  # repository roots both. A mount behind an option that machine doesn't
  # turn on isn't seen, so turn its option on there too; mounts a machine's
  # own repository adds aren't seen either.
  mountFaults = faultsOf pages mounts;

  # The network site as localSite builds it, from its folder and from that
  # folder as a string, which carries no pages, so the rules are seen to
  # hold of what localSite says and not only of a stand-in.
  network =
    src:
    sites.localSite {
      name = "network";
      inherit src;
      webKit = pkgs.emptyDirectory;
      looks = { };
    };
  shared = sites.shared pkgs.emptyDirectory;

  # The mount rules hold of a mount built each other way, so the gate is
  # seen to fail closed rather than taken to: each case `refused` must be
  # refused, against all the pages but where it says, for the reason it
  # names, and `passed` passes.
  unsourcedFault = "mount no localSite";
  unnamedFault = "in heads.nix's pages";
  unpagedFault = "name no page";
  refused = {
    "no mounts" = [
      pages
      { }
      "mounts no page"
    ];
    "only what the sites share" = [
      pages
      { local-sites = shared; }
      "mounts no page"
    ];
    "a plain folder" = [
      pages
      { x = pkgs.emptyDirectory; }
      unsourcedFault
    ];
    "a plain folder beside the network site" = [
      pages
      {
        network = network ../net/site;
        x = pkgs.emptyDirectory;
      }
      unsourcedFault
    ];
    "a plain folder under the shared kit's name" = [
      pages
      { local-sites = pkgs.emptyDirectory; }
      unsourcedFault
    ];
    "a derivation built from a source" = [
      pages
      { x.src = ../mycelium/forage; }
      unsourcedFault
    ];
    "a localSite built from its folder as a string" = [
      pages
      { network = network "${../net/site}"; }
      unsourcedFault
    ];
    "a localSite whose page pages doesn't name" = [
      (removeAttrs pages [ "Network" ])
      { network = network ../net/site; }
      unnamedFault
    ];
    "a mount naming no page that isn't the shared kit" = [
      pages
      { x.localSitePages = [ ]; }
      unpagedFault
    ];
    "a mount naming no page beside the network site" = [
      pages
      {
        network = network ../net/site;
        x = pkgs.emptyDirectory // {
          localSitePages = [ ];
        };
      }
      unpagedFault
    ];
    "a second page pages doesn't name" = [
      pages
      {
        x.localSitePages = [
          pages.Forage
          ./nowhere/cell.html
        ];
      }
      unnamedFault
    ];
  };
  passed = {
    network = network ../net/site;
    local-sites = shared;
  };
  ruleFaults =
    map
      (
        case:
        "the gate's own mount rules let ${case} through, or refuse it for another reason, so its page would go unjudged"
      )
      (
        lib.attrNames (
          lib.filterAttrs (
            _: case:
            !(lib.any (lib.hasInfix (lib.elemAt case 2)) (faultsOf (lib.elemAt case 0) (lib.elemAt case 1)))
          ) refused
        )
      )
    ++
      lib.optional (faultsOf pages passed != [ ])
        "the gate's own mount rules refuse localSite's network and the shared kit: ${lib.concatStringsSep "; " (faultsOf pages passed)}"
    ++
      lib.optional (passed.network.localSitePages != [ pages.Network ])
        "localSite names the network site's pages otherwise than its one index.html, so a page it serves may go unjudged (a page added to that site is named here, in this check, as in pages)";

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
    "Forage"
    "Machines"
    "Map"
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
