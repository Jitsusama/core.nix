# The checks of last-seconds, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: a record read back as written, a torn or
  # flipped one never read, the ring wrapping and going on after a restart,
  # each source found by its name and read as what grew, how each boot's
  # record ended, and the recorder run whole, signalled as the sleep's hook
  # does, recording each sleep whatever came before it.
  last-seconds-keeps-its-ring = pkgs.callPackage ./package.nix { };

  # The sleep's hook, as writeShellApplication runs it, against stand-ins
  # of systemctl, setpriv and the reader: it lets a sleep go once a record
  # newer than the last says it, never on the last sleep's, gives up at two
  # seconds, kills a look that hangs, and tells the recorder it woke. The VM
  # test proves it on a real sleep; this proves the cases it doesn't reach.
  last-seconds-sleeps-wait-for-the-record =
    pkgs.runCommandLocal "last-seconds-sleeps-wait-for-the-record" { }
      ''
        ${pkgs.bash}/bin/bash ${./tests/the-sleep-hook-waits-for-its-record.sh} ${./sleeps.sh} ${
          pkgs.lib.makeBinPath [
            pkgs.coreutils
            pkgs.jq
          ]
        }
        touch $out
      '';

  # clippy finds nothing in last-seconds, its tests included.
  last-seconds-passes-clippy = passesClippy "last-seconds-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
