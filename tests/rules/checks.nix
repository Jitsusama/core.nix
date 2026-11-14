# The rules held over every file, run on this repository with its waiting
# lists, which checks.nix gathers with every other folder's; README.md beside
# this says what each rule holds.
{
  exampleLaptop,
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
# The example laptop is a NixOS machine, which a Mac doesn't evaluate.
// onLinux {
  # Every daemon keeps docs/daemons.md's convention, so `ps`, systemctl and
  # the socket all say one name and every client asks the same way. It reads
  # the example laptop's account, which carries every module Linux has, so a
  # daemon no role brings, such as Slack's, is held too. Those that
  # broke a point before the rule wait in tests/rules/daemons-agree-waiting.nix,
  # with the points each still breaks, and the check fails once one keeps a
  # point it waits on, so the list only shrinks.
  daemons-agree = self.lib.daemonsAgree pkgs {
    src = ../..;
    home = exampleLaptop.config.home-manager.users.joel;
    waiting = import ./daemons-agree-waiting.nix;
  };

  # Every command keeps docs/commands.md's convention, so learning one teaches
  # them all and an agent reads each the same way. Each on the terminal kit is
  # run for its help, as JSON and in a terminal at four widths, which it has to
  # fit (docs/commands.md, Width and What Gives Way); each script whose help
  # is in our shape is run for it at the same widths. lag-watch, the first,
  # builds only on Linux, as
  # github-broker and session-backup do. Those
  # that broke a point before the rule wait in
  # tests/rules/commands-agree-waiting.nix, with the points each still
  # breaks, and the check fails once one keeps a point it waits on, so the
  # list only shrinks.
  commands-agree = self.lib.commandsAgree pkgs {
    src = ../..;
    commands =
      let
        deviceCheck = pkgs.callPackage ../../modules/device-check/package.nix { };
        google = pkgs.callPackage ../../modules/google/package.nix { };
        slack = pkgs.callPackage ../../modules/slack/package.nix { };
        pineapple = pkgs.callPackage ../../modules/pineapple/package.nix { };
        recentFiles = pkgs.callPackage ../../modules/open-with/recent-files.nix { };
        # A script whose help is in our shape, run as its module's wrapper
        # runs it: `before` sets what the wrapper sets, and `words` go ahead
        # of a person's, as the wrapper's own do. Only its help is asked.
        script =
          name: file:
          {
            before ? "",
            words ? "",
          }:
          pkgs.writeShellScriptBin name ''
            ${before}
            exec ${pkgs.lib.getExe pkgs.bash} ${file} ${words} "$@"
          '';
      in
      {
        camera = script "camera" ../../modules/camera/camera.sh { };
        disk-order = script "disk-order" ../../modules/slices/disk-order.sh { };
        ergodox = script "ergodox" ../../modules/ergodox/ergodox.sh { };
        google-opens = script "google-opens" ../../modules/google/opens.sh {
          words = "a-window an-opener";
        };
        lock-for-sleep = script "lock-for-sleep" ../../modules/swayidle/lock-for-sleep.sh { };
        lock-screen = script "lock-screen" ../../modules/swayidle/lock-screen.sh { };
        niri-deadline = script "niri-deadline" ../../modules/niri/niri-deadline.sh { };
        se-keys = script "se-keys" ../../modules/secure-enclave-signing/se-keys.sh { };
        shots-changed = script "shots-changed" ../../tools/shots-changed/shots-changed.sh { };
        sign = script "sign" ../../modules/signing/sign.sh { before = "export name=sign"; };
        ssh-askpass = script "ssh-askpass" ../../modules/quickshell/askpass.sh { };
        swayidle-away = script "swayidle-away" ../../modules/swayidle/away.sh { };
        yubikey-keys = script "yubikey-keys" ../../modules/yubikey-signing/yubikey-keys.sh { };
        bluetooth-again = pkgs.callPackage ../../modules/bluetooth-again/package.nix { };
        chrome-pack-extension = pkgs.callPackage ../../modules/chrome/pack-extension/package.nix { };
        chrome-open-tabs = pkgs.callPackage ../../modules/chrome/open-tabs/host/package.nix { };
        chrome-recent-sites = pkgs.callPackage ../../modules/chrome/recent-sites/package.nix { };
        chrome-session = pkgs.callPackage ../../modules/chrome/session/package.nix { };
        claude-plan-feed = pkgs.callPackage ../../modules/spend-watch/feeds/claude/package.nix { };
        claude-statusline = pkgs.callPackage ../../modules/claude/statusline/package.nix { };
        clock-follows = pkgs.callPackage ../../modules/quickshell/clock-follows/package.nix { };
        code-health = pkgs.callPackage ../../modules/code-health/package.nix { };
        collector = pkgs.callPackage ../../modules/collector/package.nix { };
        console-font = pkgs.callPackage ../../modules/console/font/package.nix { };
        device-check = deviceCheck;
        device-check-disk = deviceCheck;
        disk-guard = pkgs.callPackage ../../modules/disk-guard/package.nix { };
        eero = pkgs.callPackage ../../modules/eero/package.nix { };
        file-preview = pkgs.callPackage ../../modules/open-with/file-preview.nix { };
        firmware-notice = pkgs.callPackage ../../modules/firmware-notice/package.nix { };
        firmware-settings = pkgs.callPackage ../../modules/firmware-settings/package.nix { };
        gcloud-broker = pkgs.callPackage ../../modules/gcloud-broker/package.nix { };
        github-broker = pkgs.callPackage ../../modules/github-broker/package.nix { };
        inherit google;
        google-people = google;
        google-rows = google;
        ir-camera = pkgs.callPackage ../../modules/ir-camera/package.nix { };
        key-counts = pkgs.callPackage ../../modules/key-counts/package.nix { };
        lag-watch = pkgs.callPackage ../../modules/lag-watch/package.nix { };
        lid-docked = pkgs.callPackage ../../modules/lid/docked/package.nix { };
        load-proof = pkgs.callPackage ../../modules/load-proof/package.nix { };
        memory-modules = pkgs.callPackage ../../modules/memory-modules/package.nix { };
        open-now = recentFiles;
        open-with = pkgs.callPackage ../../modules/open-with/package.nix { };
        pause-for-calls = pkgs.callPackage ../../modules/spotify/pause-for-calls/package.nix { };
        pi-packages = pkgs.callPackage ../../modules/pi-packages/package.nix { };
        pi-spawner = pkgs.callPackage ../../modules/pi/spawner/package.nix { };
        inherit pineapple;
        pineapple-slack = pineapple;
        plymouth-waits = pkgs.callPackage ../../modules/plymouth/waits/package.nix { };
        quickshell-hands-over = import ../../modules/quickshell/hands-over.nix { inherit pkgs; };
        recent-files = recentFiles;
        session-backup = pkgs.callPackage ../../modules/session-backup/package.nix { };
        shell-weighs = pkgs.callPackage ../../modules/quickshell/shell-weighs/package.nix { };
        shutdown-status = pkgs.callPackage ../../modules/systemd/shutdown-status/package.nix { };
        slack-people = slack;
        slack-unreads = slack;
        spend-watch = pkgs.callPackage ../../modules/spend-watch/package.nix { };
        spotify-recent = pkgs.callPackage ../../modules/spotify/spotify-recent/package.nix { };
        vm-first = pkgs.callPackage ../../modules/nix-daemon/vm-first/package.nix { };
        without-syncfs = pkgs.callPackage ../../modules/nix-daemon/without-syncfs/package.nix { };
        xwayland-satellite = pkgs.callPackage ../../modules/niri/xwayland-satellite.nix { };
        zathura-command = pkgs.callPackage ../../modules/zathura/command/package.nix { };
        zathura-next-document = pkgs.callPackage ../../modules/zathura/next-document/package.nix { };
      };
    waiting = import ./commands-agree-waiting.nix;
    views =
      let
        # A stand-in a script builds, where kept files would be folders a
        # dozen deep with no note each.
        built = name: script: builtWith name [ ] script;
        # The same, with the programs it needs on PATH.
        builtWith =
          name: inputs: script:
          pkgs.runCommand name { nativeBuildInputs = inputs; } ''
            ${pkgs.runtimeShell} ${script} $out
          '';
        history = builtWith "a-profile-with-history" [
          pkgs.sqlite
        ] ../../modules/chrome/recent-sites/tests/a-profile-with-history.sh;
      in
      {
        # Six sites and three downloads, one site and one file long-named,
        # and what's left out of each.
        chrome-recent-sites = [
          {
            state = history;
            args = [ "@HERE@" ];
          }
          {
            state = history;
            args = [
              "downloads"
              "@HERE@"
            ];
          }
        ];
        # Five tabs in two windows, one titled past any width and one of
        # Chrome's own pages.
        chrome-session = [
          {
            state = built "a-profile-with-tabs" ../../modules/chrome/session/tests/a-profile-with-tabs.sh;
            args = [ "@HERE@" ];
          }
        ];
        # A clock moved hours ago, with a zone put back from and held.
        clock-follows = [ { state = ../../modules/quickshell/clock-follows/tests/a-moved-clock; } ];
        # A laptop with a failed device, a missing one and a disk's fault.
        device-check = [
          {
            state = built "a-failing-laptop" ../../modules/device-check/tests/a-failing-laptop.sh;
            env = {
              DEVICE_CHECK_ROOT = "@HERE@/root";
              DEVICE_CHECK_EXPECTED = "@HERE@/expected";
              DEVICE_CHECK_STATE = "@HERE@/found";
              DEVICE_CHECK_NOTIFY = "true";
              DEVICE_CHECK_SUSPENDED = "0";
            };
          }
        ];
        # The made-up home network eero's golden pages are drawn from, as
        # the daemon keeps it, read long ago, so each page opens saying the
        # daemon may have stopped; each view is set from it, with a name
        # for those of one node, device or profile.
        eero =
          let
            state = pkgs.runCommand "a-home-network" { } ''
              mkdir -p $out/runtime/eero
              cp ${../../modules/eero/home-network.json} $out/runtime/eero/home-network.json
            '';
            view = args: { inherit args state; };
          in
          map view [
            [ ]
            [ "nodes" ]
            [
              "node"
              "Study"
            ]
            [ "mesh" ]
            [ "devices" ]
            [
              "device"
              "unknown computer 1"
            ]
            [ "internet" ]
            [ "speeds" ]
            [ "firmware" ]
            [ "radio" ]
            [ "traffic" ]
            [ "profiles" ]
            [
              "profile"
              "profile 1"
            ]
            [ "events" ]
          ];
        # Firmware off two of its declared settings, which a bare run names
        # and exits 1 for, as diff does.
        firmware-settings = [
          {
            state = built "a-laptop-off-its-settings" ../../modules/firmware-settings/tests/a-laptop-off-its-settings.sh;
            env = {
              FIRMWARE_ATTRIBUTES = "@HERE@/fw";
              FIRMWARE_SETTINGS = "@HERE@/declared";
              FIRMWARE_SETTINGS_WAITING = "@HERE@/waiting";
            };
            exits = 1;
          }
        ];
        # An hour across two generations and every load, its last window
        # stalled under a unit with a long name.
        lag-watch =
          let
            state = built "a-busy-hour" ../../modules/lag-watch/a-busy-hour.sh;
          in
          [
            { inherit state; }
            {
              args = [ "report" ];
              inherit state;
            }
          ];
        # A loaded reading that fell short of idle in five signals, which
        # judge names and exits 1 for.
        load-proof = [
          {
            args = [
              "judge"
              "@HERE@/idle.txt"
              "@HERE@/busy.txt"
            ];
            state = ../../modules/load-proof/tests/a-loaded-reading;
            exits = 1;
          }
        ];
        # One package swapped in and one not yet, under a hold whose reason
        # wraps, put in place first as a person would.
        pi-packages = [
          {
            args = [
              "--config"
              "@HERE@/settings.json"
            ];
            state = ../../modules/pi-packages/tests/a-held-swap;
            first = [
              [
                "--config"
                "@HERE@/settings.json"
                "hold"
                "the"
                "launcher's"
                "demo"
                "at"
                "three,"
                "so"
                "nothing"
                "moves"
                "under"
                "it"
              ]
            ];
          }
        ];
        # A machine with logs still to send, and logs sent with no row.
        session-backup = [
          {
            args = [
              "status"
              "--config"
              "@HERE@/settings.json"
            ];
            state = ../../modules/session-backup/tests/a-machine-behind;
          }
        ];
        # A week of three sessions, one serving a quest with a long title
        # and rebuilding its context after a pause, imported first as a
        # person would before reading a report.
        spend-watch =
          let
            state = ../../modules/spend-watch/tests/a-busy-week;
            first = [ [ "import" ] ];
          in
          [
            { inherit state first; }
            {
              args = [
                "report"
                "30"
                "days"
                "by"
                "quest"
              ];
              inherit state first;
            }
            {
              args = [
                "report"
                "waste"
              ];
              inherit state first;
            }
            {
              args = [ "budgets" ];
              inherit state first;
            }
          ];
        # A guard whose disk is nearly full, with the longest holders and
        # clears it says, which is when its status is read, and the long
        # paths its journal names.
        disk-guard =
          let
            state = ../../modules/disk-guard/tests/a-full-disk;
          in
          [
            { inherit state; }
            {
              args = [ "status" ];
              inherit state;
            }
            {
              args = [ "journal" ];
              inherit state;
            }
          ];
      };
  };
}
