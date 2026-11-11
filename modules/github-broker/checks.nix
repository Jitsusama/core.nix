# The checks of github-broker, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # github-broker's tests run as it builds against a stand-in for GitHub: a
  # token signed with the App's key, narrowed to reading code and writing
  # pull requests and issues, asked for once while it lasts, a refusal said,
  # and an App not made yet.
  github-broker-mints-a-narrow-token = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in github-broker, its tests included.
  github-broker-passes-clippy = passesClippy "github-broker-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # Every workstation account's broker listens where only the account can
  # ask, and runs in the background.
  github-broker-answers-only-its-account =
    let
      user = account workstation;
      listens = user.systemd.user.sockets.github-broker.Socket;
      service = user.systemd.user.services.github-broker.Service;
    in
    holds "github-broker-answers-only-its-account"
      "github-broker's socket lets other accounts ask for a token, or it runs outside background.slice."
      (
        listens.SocketMode == "0600"
        && listens.ListenStream == "%t/github-broker.sock"
        && service.Slice == "background.slice"
      );
}
