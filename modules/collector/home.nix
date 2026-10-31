# The collector, which keeps the traces the account's programs send it so a
# moment can be read back afterwards; src/main.rs says how it's asked.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  # `collector at` and `collector trace`, which find the store and socket
  # the service uses by themselves.
  home.packages = [ package ];

  systemd.user.services.collector = {
    Unit.Description = "Keeps the traces the account's programs send";
    Service = {
      # Its socket is made in the account's runtime folder, so senders find
      # it at $XDG_RUNTIME_DIR/collector/lines.sock.
      ExecStart = "${lib.getExe package} serve";
      # Whatever the account runs comes first; this only keeps what it's
      # sent, and a sender drops a batch rather than wait for it.
      Slice = "background.slice";
      RuntimeDirectory = "collector";
      Restart = "on-failure";
      RestartSec = "5s";
    };
    Install.WantedBy = [ "default.target" ];
  };
}
