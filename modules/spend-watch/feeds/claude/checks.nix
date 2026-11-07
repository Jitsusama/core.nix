# The checks of spend-watch's feed for a Claude plan, which checks.nix gathers
# with every other folder's; its opening comment says what each kind of check
# proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# The feed reads Claude Code's sign-in from the file Linux keeps it in, and
# builds only there.
onLinux {
  # The feed's tests run as it builds, against recorded sign-ins and answers:
  # a plan's windows are caps of a share, labelled for the plan, as the
  # contract example beside them says; a window the plan lacks or that has
  # rolled isn't said; nothing signed in is no fault, an expired sign-in
  # stalls without asking, and an answer with nothing usable is an error;
  # and it reads at spend-watch's pace, backing off while the endpoint fails
  # or asks it to wait.
  claude-plan-feed-says-how-the-plan-stands = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the feed, its tests included.
  claude-plan-feed-passes-clippy = passesClippy "claude-plan-feed-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
