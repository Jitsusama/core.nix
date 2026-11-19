# What a switch refuses in the declared rows' homes, read as the homes'
# readers read them (modules/system/src/homes.rs and the web-design
# skill's modules/theme/web/skill/homes.js): a kind two declared sites are
# home to, a pattern the readers would drop, and a kind one site gives two
# patterns. home.nix asserts each is empty;
# local-sites-rows-homes-read-as-readers-do holds them to rows that trip each
# rule.
{ lib }:
rows:
let
  # Whether an address's bytes are visible ASCII with no backslash, double
  # quote, angle bracket or backtick, as both readers hold a pattern to.
  plain =
    address:
    let
      visible = builtins.match "[!-~]*" address != null;
      breaking = builtins.match ".*[\\\"<>`].*" address != null;
    in
    visible && !breaking;
  # A pattern the readers keep: once its row's path is taken off the front,
  # a fragment on the row's own page with a place for the thing's name.
  kept =
    path: address:
    let
      fragment = fragmentOf path address;
    in
    lib.hasPrefix "#" fragment && lib.hasInfix "{name}" fragment && plain fragment;
  # A pattern as the readers read it, on its row's own page, however it's
  # written.
  fragmentOf = path: lib.removePrefix "${path}/";
  # Whether a row's home is a pattern the readers would keep, were it the
  # first its row gives its kind.
  keeps = path: home: !lib.isString home && home ? kind && home ? address && kept path home.address;
  # Each of a row's homes, a kind's name or a kind with its pattern, and
  # whether the readers would take it as a home. Both readers take a row's
  # first kept pattern for a kind and pass over any later one without a
  # word, so a later one is marked `again`.
  claims = lib.concatLists (
    lib.mapAttrsToList (
      path: row:
      lib.imap0 (
        at: home:
        if lib.isString home then
          {
            inherit path;
            kind = home;
            homes = true;
            again = false;
          }
        else
          let
            first = lib.findFirst (earlier: keeps path earlier && earlier.kind == home.kind) null (
              lib.take at row.homes
            );
          in
          {
            inherit path;
            kind = home.kind or null;
            address = home.address or null;
            homes = keeps path home && first == null;
            again = keeps path home && first != null;
            fragment = fragmentOf path home.address;
          }
      ) row.homes
    ) rows
  );
  # The patterns the readers would keep, were each its kind's first, by the
  # site and kind they're given for.
  keptPatterns = lib.groupBy (claim: "${claim.path}'s ${claim.kind}") (
    lib.filter (claim: claim ? fragment && (claim.homes || claim.again)) claims
  );
  # Patterns in their order, each read the same way kept once.
  distinct = lib.foldl' (
    kept: claim:
    if lib.any (each: each.fragment == claim.fragment) kept then kept else kept ++ [ claim ]
  ) [ ];
  # The sites each kind is home on. A row may give its own kind both by
  # name and by pattern, which both readers take as the pattern's, so only
  # a second site counts.
  sites = lib.mapAttrs (_: claims: lib.unique (map (claim: claim.path) claims)) (
    lib.groupBy (claim: claim.kind) (lib.filter (claim: claim.homes) claims)
  );
in
{
  # Kinds more than one declared site is home to. The web-design skill's
  # reader calls a kind's second home a fault. The system daemon merges two
  # rows naming a kind, thing by thing, but gives a pattern's kind wholly to
  # the first row claiming it, so there one site's links would go.
  homedTwice = lib.mapAttrsToList (kind: paths: "${kind} (${lib.concatStringsSep ", " paths})") (
    lib.filterAttrs (_: paths: lib.length paths > 1) sites
  );
  # Patterns the readers drop. The skill's reader calls each a fault, and
  # where one is its row's only claim on a kind, the daemon leaves the kind
  # no home there without a word, so its links would simply go.
  dropped = map (
    claim:
    "${claim.path}'s ${if claim.kind == null then "pattern with no kind" else claim.kind}, ${
      if claim.address == null then "with no address" else builtins.toJSON claim.address
    }"
  ) (lib.filter (claim: !claim.homes && !claim.again) claims);
  # Kinds a site gives more than one pattern the readers would keep, each
  # pattern named once and the first first. Neither reader says a word of
  # it: each takes the first, so the others' links are never drawn. A
  # pattern given again, however it's written, draws the first's links, so
  # it's no second.
  patternedTwice = lib.concatLists (
    lib.mapAttrsToList (
      named: patterns:
      let
        written = map (claim: builtins.toJSON claim.address) (distinct patterns);
      in
      lib.optional (lib.length written > 1) (
        "${named}, ${lib.concatStringsSep ", " (lib.init written)} and ${lib.last written}"
      )
    ) keptPatterns
  );
}
