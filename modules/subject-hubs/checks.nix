# The checks of subject-hubs, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: the node's answer to the list of hubs names
  # one hub or several and an error none, read from the line with the
  # list's id; a need and a subject are read into words as the harness
  # reads them, common words left out and joined ids kept whole; the
  # narrowest live subject said, as the harness judges a word said, takes a
  # thing first; the socket is the realm's named or kept; and each
  # need is kept once in its UTC day's file, in the harness's shape.
  subject-hubs-reads-hubs-and-keeps-needs = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in subject-hubs, its tests included.
  subject-hubs-passes-clippy = passesClippy "subject-hubs-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
