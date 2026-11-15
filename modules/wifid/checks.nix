# The checks of wifid, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  services = (bareNixos [ self.nixosModules.wifid ]).config.systemd.services;
in
# wifid talks to wpa_supplicant over D-Bus and its window is WebKitGTK's, so
# both are Linux's alone.
onLinux {
  # wifid's tests run as it builds: what it hears of the air and offers,
  # what each network asks to be joined by, the address it shows each
  # network, the networks and secrets it keeps, and how it tells a network
  # that wants a sign-in. They ran before only inside a machine's build.
  # Its tests read the launcher's Wi-Fi words from daemon/tests/words.json,
  # which the-daemons-test-the-launchers-words holds to the launcher's own.
  wifid-reads-the-air = pkgs.callPackage ./package.nix { };

  # The sign-in window's tests run as it builds: its page's proxy lets on
  # only the window and only connects, and it reads from wifid which card
  # wants a sign-in until none does.
  wifid-sign-in-reads-its-network = pkgs.callPackage ./sign-in/package.nix { };
  # clippy finds nothing in wifid or its sign-in window, their tests
  # included. The window's package carries what both members need, so it
  # reads the whole workspace.
  wifid-passes-clippy = passesClippy "wifid-passes-clippy" (
    pkgs.callPackage ./sign-in/package.nix { }
  );

  # What tells wifid of calls has its tests run as it builds: it says
  # nothing is on at its start, then only a change or a call still on a
  # minute later, and wifid's rows socket hears exactly a calling act.
  wifid-calls-tells-wifid = pkgs.callPackage ./calls/package.nix { };
  # clippy finds nothing in it, its tests included.
  wifid-calls-passes-clippy = passesClippy "wifid-calls-passes-clippy" (
    pkgs.callPackage ./calls/package.nix { }
  );

  # A switch restarts wifid and wpa_supplicant in place, never stopping
  # them before it activates, so a switch whose later units hang still
  # brings both back. On 14 Nov 2026 one hung starting the account's
  # home-manager unit, and the units stopped beforehand, wifid among them,
  # were never started again: the card stayed off its network until a
  # reboot.
  wifid-comes-back-from-a-stalled-switch =
    holds "wifid-comes-back-from-a-stalled-switch"
      "A switch would stop wifid or wpa_supplicant first, so a hang leaves them down."
      (
        services.wifid.restartIfChanged
        && !services.wifid.stopIfChanged
        && services.wpa_supplicant.restartIfChanged
        && !services.wpa_supplicant.stopIfChanged
      );
}
