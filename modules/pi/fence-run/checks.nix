# The checks of pi-fence, the store's launcher for one command in pi's own
# fence (package.nix).
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # pi-fence reads how it was asked, and where it can't lay the fence, as in
  # a build with neither bubblewrap nor landrun, it refuses and says why
  # rather than running the command unfenced, in either kind.
  pi-fence-never-runs-unfenced = pkgs.runCommandLocal "pi-fence-never-runs-unfenced" { } ''
    ${pkgs.lib.getExe pkgs.bash} ${./never-runs-unfenced.sh} \
      ${pkgs.lib.getExe (pkgs.callPackage ./package.nix { })}
    touch $out
  '';
}
