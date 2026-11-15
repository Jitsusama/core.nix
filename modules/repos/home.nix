# repos lists every repository on the machine once, with every checkout of
# it and every tree cut from each (src/lib.rs says how). It reads git's own
# files as it runs, so it keeps no daemon and no record. Where it looks is
# the one setting that differs between machines: a machine whose trees lie
# in a folder of their own, under no checkout, adds that folder.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  repos = pkgs.callPackage ./package.nix { };
  # The command with its roots set, as a default a caller may still set
  # for one run.
  wrapped = pkgs.symlinkJoin {
    name = "repos-${repos.version}";
    paths = [ repos ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/repos \
        --set-default REPOS_ROOTS ${lib.escapeShellArg (lib.concatStringsSep ":" config.jitsusama.repos.roots)}
    '';
    inherit (repos) meta;
  };
in
{
  options.jitsusama.repos.roots = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ "${config.home.homeDirectory}/src" ];
    description = ''
      The folders repos looks under for checkouts, four folders deep. A
      tree is found from the checkout it was cut from, wherever it lies,
      so only a folder of trees whose checkout is under no root needs
      naming.
    '';
  };

  config.home.packages = [ wrapped ];
}
