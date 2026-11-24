# The checks of lag-watch, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
# lag-watch reads Linux's pressure files and threads, and builds only there.
onLinux {
  # lag-watch's tests run as it builds: the desktop or apps waiting is
  # trouble and work waiting on work isn't, niri slow to answer is, the
  # watched threads are found by name, a call's worst thread is named for
  # Tuple and Chrome apart, and the report sums each generation by its
  # load.
  lag-watch-tells-who-waited = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in lag-watch, its tests included.
  lag-watch-passes-clippy = passesClippy "lag-watch-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # Slack's and Chrome's time blocked is read only while the kernel keeps
  # it, which needs both halves: a machine with a screen asking for it, and
  # core's kernel built able to. Without either, every row reads no time
  # blocked and nothing fails. Only the kernel's configuration is built,
  # which the kernel's own checks build too.
  lag-watch-can-read-time-blocked =
    let
      asked = toString (graphical.config.boot.kernel.sysctl."kernel.sched_schedstats" or "nothing");
    in
    pkgs.runCommandLocal "lag-watch-can-read-time-blocked" { } ''
      $BASH ${./can-read-time-blocked.sh} '${asked}' ${self.packages.x86_64-linux.kernel.configfile}
      touch $out
    '';
}
