# The checks of wifid, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# wifid talks to wpa_supplicant over D-Bus and its window is WebKitGTK's, so
# both are Linux's alone.
onLinux {
  # wifid's tests run as it builds: what it hears of the air and offers,
  # what each network asks to be joined by, the address it shows each
  # network, the networks and secrets it keeps, and how it tells a network
  # that wants a sign-in. They ran before only inside a machine's build.
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
}
