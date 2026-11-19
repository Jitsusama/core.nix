# What a switch refuses in the declared rows' homes, read as the homes'
# readers read them (modules/system/src/homes.rs and the web-design
# skill's modules/theme/web/skill/homes.js): a kind two declared sites are
# home to, and a pattern the readers would drop. home.nix asserts both are
# empty; local-sites-rows-homes-read-as-readers-do holds them to rows that trip
# each rule.
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
  # Each of a row's homes, a kind's name or a kind with its pattern, and
  # whether the readers would take it as a home.
  claims = lib.concatLists (
    lib.mapAttrsToList (
      path: row:
      map (
        home:
        if lib.isString home then
          {
            inherit path;
            kind = home;
            homes = true;
          }
        else
          {
            inherit path;
            kind = home.kind or null;
            address = home.address or null;
            homes = home ? kind && home ? address && kept path home.address;
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
  ) (lib.filter (claim: !claim.homes) claims);
}
