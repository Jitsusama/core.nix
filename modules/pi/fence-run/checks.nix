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
  pi-fence-never-runs-unfenced =
    let
      fence = pkgs.callPackage ./package.nix { };
    in
    pkgs.runCommandLocal "pi-fence-never-runs-unfenced" { } ''
      export HOME=$PWD/home
      mkdir -p "$HOME" work
      cd work
      status=0
      ${pkgs.lib.getExe fence} --kind loose -- true 2> asked || status=$?
      if [ "$status" != 2 ] || ! grep -q "usage: pi-fence" asked; then
        echo "a wrong kind wasn't refused with the usage ($status)" >&2
        exit 1
      fi
      for kind in fence strict; do
        status=0
        ${pkgs.lib.getExe fence} --kind "$kind" -- touch ran 2> said || status=$?
        if [ "$status" != 126 ] || [ -e ran ] || ! grep -q "^pi-fence: " said; then
          echo "--kind $kind ran unfenced, or refused without a reason ($status)" >&2
          cat said >&2
          exit 1
        fi
      done
      touch $out
    '';
}
