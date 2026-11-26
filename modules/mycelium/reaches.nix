# The options naming the machines a realm's node reaches through a
# provider's program, and the providers that may make them (home.nix's
# `vias` and `providers`, for the home realm and each further one). They
# change only when how a provider's program is named does, apart from how
# the node is run.
{ lib }:
let
  # A machine a provider's program reaches, as `PROGRAM:PLACE`: the program
  # by its absolute path, since a Mac's agent runs the node with PATH
  # /usr/bin:/bin and Linux's PATH isn't one to count on, and the place it
  # names. Neither takes whitespace, `%` or `$`, which systemd expands even
  # inside quotes, and the place doesn't start with `-`, so the program
  # can't read it as a flag.
  via = lib.types.strMatching "/[^:[:space:]%$]+:[^-:[:space:]%$][^[:space:]%$]*";
  # A provider's program by its absolute path, for the reason a via's is;
  # it takes no words of its own, since the node adds what it asks.
  provider = lib.types.strMatching "/[^:[:space:]%$]+";
in
{
  vias = lib.mkOption {
    type = lib.types.listOf via;
    default = [ ];
    example = lib.literalExpression ''[ "''${cistern-link}/bin/cistern-link:sandbox-1" ]'';
    description = ''
      The machines of the realm this machine dials and keeps linked through a
      provider's program, as `PROGRAM:PLACE` with the program by its absolute
      path, for a machine SSH can't reach, such as one a provider lent.
      mycelium's PROTOCOL.md says what the program is asked.
    '';
  };
  providers = lib.mkOption {
    type = lib.types.listOf provider;
    default = [ ];
    example = lib.literalExpression ''[ "''${cistern-cell}/bin/cistern-cell" ]'';
    description = ''
      The programs of providers that may make this realm's machines, each by
      its absolute path. The node asks each what it offers, and makes a
      machine from an offer only for a start no machine of the realm takes,
      and from one that costs money only where the start allows it.
      mycelium's PROTOCOL.md says what the program is asked.
    '';
  };
}
