# The checks of git-engine, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # git-engine answers as gitstatusd does across the states a prompt meets,
  # counting everything and stopping at one, some of them watched by git's
  # file system monitor, and as git does where gitstatusd can't read the
  # repository (modules/git-engine/tests/agrees.zsh). The plugin wants job
  # control, so it runs on a terminal of script's.
  git-engine-agrees-with-gitstatusd =
    let
      engine = pkgs.callPackage ./package.nix { };
      plugin = "${pkgs.gitstatus}/share/gitstatus";
    in
    pkgs.runCommandLocal "git-engine-agrees-with-gitstatusd"
      {
        nativeBuildInputs = [
          pkgs.git
          pkgs.util-linux
          pkgs.zsh
        ];
      }
      ''
        export HOME=$TMPDIR/home
        script -q -e -c "zsh -f -i ${./tests/agrees.zsh} ${plugin} \
          ${lib.getExe' pkgs.gitstatus "gitstatusd"} ${lib.getExe engine} $TMPDIR/work" \
          agrees.log < /dev/null > /dev/null || { tr -d '\r' < agrees.log >&2; exit 1; }
        tr -d '\r' < agrees.log | grep checked
        touch $out
      '';
}
// onLinux {
  # clippy finds nothing in git-engine, its tests included.
  git-engine-passes-clippy = passesClippy "git-engine-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
