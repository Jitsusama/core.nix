# The checks of file-link-keys, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
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
  # Where there's no TPM, a realm's key is a file in a private folder of its
  # own, outside ~/.ssh where the account's own agent would load it, served
  # on the socket its node is given by an agent holding only it. An account
  # naming no realm gets none.
  a-realms-link-key-in-a-file-stays-apart =
    let
      home = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.file-link-keys
            { jitsusama.linkKeys = [ "personal" ]; }
          ];
        }
      ]);
      none = account (bareNixos [
        { home-manager.sharedModules = [ self.homeModules.file-link-keys ]; }
      ]);
      folder = "/home/someone/.local/share/links/personal";
      agent = home.systemd.user.services.personal-link-agent;
    in
    holds "a-realms-link-key-in-a-file-stays-apart"
      "A realm's link key in a file isn't served on its own socket by an agent holding only it, it's kept where the account's own agent loads it, its folder isn't private, or an account naming no realm gets one."
      (
        home.systemd.user.sockets.personal-link.Socket.ListenStream == "%t/personal-link.sock"
        && lib.hasSuffix " -q ${folder}/personal-link" agent.Service.ExecStartPost
        && agent.Unit.ConditionPathExists == "${folder}/personal-link"
        && builtins.elem "d ${folder} 0700 - - -" home.systemd.user.tmpfiles.rules
        && !(none.systemd.user.sockets ? personal-link)
      );
}
