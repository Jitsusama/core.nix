# wifid's sign-in window, for the account's session: the shell opens it
# when Joel asks to sign in to a network that wants it, and never by
# itself. It wears the GTK theme the session does.
{ pkgs, ... }:
{
  home.packages = [ (pkgs.callPackage ./sign-in/package.nix { }) ];
}
