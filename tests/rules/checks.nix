# The rules held over every file, run on this repository with its waiting
# lists, which checks.nix gathers with every other folder's; README.md beside
# this says what each rule holds.
{
  account,
  graphical,
  onLinux,
  pkgs,
  self,
  ...
}:
{
  # A code file past 800 lines, or a folder of more than 40 entries, says
  # why it stays whole or is split, since one that grew past them is
  # usually several things a reader has to tell apart. Those over before
  # the rule wait in tests/rules/every-long-file-says-why-waiting.nix, and
  # the check fails once one is split, says why or is gone, so the list
  # only shrinks.
  every-long-file-says-why = self.lib.everyLongFileSaysWhy pkgs {
    src = ../..;
    waiting = import ./every-long-file-says-why-waiting.nix;
  };

  # A program past five lines lives in a file of its own language rather
  # than a Nix string, since no linter, type checker or editor reads a
  # string and the values spliced into one are where quoting breaks. Those
  # in strings before the rule wait, counted by file, in
  # tests/rules/programs-live-in-their-own-files-waiting.nix, and a count
  # may only fall.
  programs-live-in-their-own-files = self.lib.programsLiveInTheirOwnFiles pkgs {
    src = ../..;
    waiting = import ./programs-live-in-their-own-files-waiting.nix;
  };

  # A test waits on the event it needs, and a fixed wait stands only for
  # "nothing happens for this long" and says so, since a guess at how long
  # a loaded machine takes fails or wastes time when it's wrong. Those
  # before the rule wait, counted by file, in
  # tests/rules/every-fixed-wait-says-why-waiting.nix, and a count may only
  # fall.
  every-fixed-wait-says-why = self.lib.everyFixedWaitSaysWhy pkgs {
    src = ../..;
    waiting = import ./every-fixed-wait-says-why-waiting.nix;
  };

  # A failure that says only "assertion failed" makes the reader run the test
  # again to learn anything, so an assertion the language can't show both
  # sides of carries a message. Those before the rule wait, counted by
  # file, in tests/rules/every-assert-says-what-was-wrong-waiting.nix, and a
  # count may only fall.
  every-assert-says-what-was-wrong = self.lib.everyAssertSaysWhatWasWrong pkgs {
    src = ../..;
    waiting = import ./every-assert-says-what-was-wrong-waiting.nix;
  };

  # A test that can't run, for want of a filesystem, a device or a tool,
  # fails rather than passing, since a skip nobody sees passes a gate that
  # proved nothing. Those before the rule wait, counted by file, in
  # tests/rules/every-test-runs-or-fails-waiting.nix, and a count may only
  # fall.
  every-test-runs-or-fails = self.lib.everyTestRunsOrFails pkgs {
    src = ../..;
    waiting = import ./every-test-runs-or-fails-waiting.nix;
  };

  # A suppression is a judgement that the linter is wrong here, and one with
  # no reason can't be checked again when the code or the linter changes.
  # Those before the rule wait, counted by file, in
  # tests/rules/every-suppression-says-why-waiting.nix, and a count may only
  # fall.
  every-suppression-says-why = self.lib.everySuppressionSaysWhy pkgs {
    src = ../..;
    waiting = import ./every-suppression-says-why-waiting.nix;
  };

  # shellcheck's optional checks for a hidden failure, a set -e turned off,
  # an uppercase variable nothing sets and a case with no default find
  # faults, but too many scripts break them for treefmt to run them yet.
  # Those findings wait, counted by script, in
  # tests/rules/every-script-meets-shellcheck-at-its-strictest-waiting.nix,
  # and a count may only fall.
  every-script-meets-shellcheck-at-its-strictest =
    self.lib.everyScriptMeetsShellcheckAtItsStrictest pkgs
      {
        src = ../..;
        waiting = import ./every-script-meets-shellcheck-at-its-strictest-waiting.nix;
        includes = [ "modules/quickshell/in-a-terminal/tests/qs" ];
      };

  # A test's function copied into a second file drifts from the first the
  # next time one is fixed. Those before the rule wait, counted by file, in
  # tests/rules/every-test-function-lives-once-waiting.nix, and a count may
  # only fall.
  every-test-function-lives-once = self.lib.everyTestFunctionLivesOnce pkgs {
    src = ../..;
    waiting = import ./every-test-function-lives-once-waiting.nix;
  };
}
# A graphical machine's account is a NixOS one, which a Mac doesn't evaluate.
// onLinux {
  # Every daemon keeps docs/daemons.md's convention, so `ps`, systemctl and
  # the socket all say one name and every client asks the same way. Those that
  # broke a point before the rule wait in tests/rules/daemons-agree-waiting.nix,
  # with the points each still breaks, and the check fails once one keeps a
  # point it waits on, so the list only shrinks.
  daemons-agree = self.lib.daemonsAgree pkgs {
    src = ../..;
    home = account graphical;
    waiting = import ./daemons-agree-waiting.nix;
  };
}
