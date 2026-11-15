# The checks of net, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # net's tests run as it builds: the record reads into the model and
  # writes back unchanged, names its schema, and holds one eero as one
  # device with both its roles.
  net-reads-the-home-network = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in net, its tests included.
  net-passes-clippy = passesClippy "net-passes-clippy" (pkgs.callPackage ./package.nix { });
}
