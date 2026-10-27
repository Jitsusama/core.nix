# The checks of google, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# google serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # google's tests run as it builds: the harness's file is read for its
  # default or first account and never written, pi's fresh token is used
  # and a stale one refreshed in memory, a refused sign-in is signed out
  # and not offered again until a new one, a revoked token is refreshed
  # past once, an unreachable Google keeps what was heard, every page of
  # meetings is joined, each meeting's times, answer and the one link it's
  # joined by are read, an invitation is answered by changing only Joel's
  # own answer, and held at once, one Google refuses changing nothing, the
  # mail that wants Joel is told newest first, each asked for once and kept
  # while Gmail fails, mail marked read or archived takes one label off and
  # wants Joel no more, the shell asks only what's read strictly, and it hears the state on
  # connecting and each change once. A stand-in on the machine answers for
  # Google.
  google-reads-the-calendar = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in google, its tests included.
  google-passes-clippy = passesClippy "google-passes-clippy" (pkgs.callPackage ./package.nix { });
}
