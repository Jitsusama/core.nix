# The checks of link-keys, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  self,
  ...
}:
onLinux {
  # A realm's key is served by an agent of its own, on the socket its node
  # is given, from a folder holding only that key and outside ~/.ssh, where
  # the account's own agent would load it too; the guide makes it with no
  # PIN. An account naming no realm gets none.
  a-realms-link-key-stays-apart =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.link-keys
            { jitsusama.linkKeys = [ "personal" ]; }
          ];
        }
      ]);
      none = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.link-keys ]; } ]);
      folder = "/home/someone/.local/share/links/personal";
      keys = lib.filter (key: key.label == "personal link") home.jitsusama.tpmKeys;
    in
    holds "a-realms-link-key-stays-apart"
      "A realm's link key isn't served on its own socket by an agent holding only it, it's kept where the account's own agent loads it, it asks for a PIN, or an account naming no realm gets one."
      (
        home.systemd.user.sockets.personal-link.Socket.ListenStream == "%t/personal-link.sock"
        && lib.hasSuffix "--key-dir ${folder}" home.systemd.user.services.personal-link.Service.ExecStart
        && keys == [
          {
            file = "${folder}/personal-link";
            label = "personal link";
            pin = false;
            github = null;
          }
        ]
        && !(lib.hasPrefix "/home/someone/.ssh" (builtins.head keys).file)
        && !(none.systemd.user.sockets ? personal-link)
        && !(lib.any (key: lib.hasSuffix " link" key.label) none.jitsusama.tpmKeys)
      );
}
