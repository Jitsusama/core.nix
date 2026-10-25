# The checks of bluetooth-links, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves. It
# speaks Linux's Bluetooth sockets, so both are Linux's alone.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: the kernel's lists of adapters and
  # connections are read, keeping only the data links that are up, with
  # who leads each, who started it and how it's secured, and what the
  # kernel says of each adapter, its address and bytes carried; each
  # command is written as the controller takes it, and its answer picked
  # out of the events, refused or not; the signal, link quality, power,
  # channel map and the adapter's version are read from reply bytes, a
  # failed one leaving its field null; each link keeps its last twenty
  # reads, starting afresh when it comes back, and each adapter its
  # traffic, a counter starting again counting from nothing; the version
  # is asked once; nothing new is written while nothing is connected; and
  # the file is replaced whole, readable by everyone.
  bluetooth-links-reads-the-replies = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  bluetooth-links-passes-clippy = passesClippy "bluetooth-links-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
