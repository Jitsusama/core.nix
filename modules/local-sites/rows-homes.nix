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
      fragment = lib.removePrefix "${path}/" address;
    in
    lib.hasPrefix "#" fragment && lib.hasInfix "{name}" fragment && plain fragment;
  # Whether a row's home is a pattern the readers would keep, were it the
  # first its row gives its kind.
  keeps = path: home: !lib.isString home && home ? kind && home ? address && kept path home.address;
  # Each of a row's homes, a kind's name or a kind with its pattern, and
  # whether the readers would take it as a home. Both readers take a row's
  # first kept pattern for a kind and pass over any later one without a
  # word, so a later one is marked as the first's `again`.
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
            again = null;
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
            again = if keeps path home && first != null then first.address else null;
          }
      ) row.homes
    ) rows
  );
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
  ) (lib.filter (claim: !claim.homes && claim.again == null) claims);
  # Kinds a site gives a second pattern the readers would keep. Neither
  # reader says a word of it: each takes the first, so the second's links
  # are never drawn.
  patternedTwice = map (
    claim:
    let
      first = builtins.toJSON claim.again;
      second = builtins.toJSON claim.address;
    in
    "${claim.path}'s ${claim.kind}, ${first} and ${second}"
  ) (lib.filter (claim: claim.again != null) claims);
}
