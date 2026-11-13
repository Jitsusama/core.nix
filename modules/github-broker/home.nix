# github-broker hands out a GitHub App's installation tokens, an hour each,
# at $XDG_RUNTIME_DIR/github-broker.sock (src/lib.rs says why). A sandbox's
# fence leaves that socket out, so what asks is the program starting a
# sandboxed command, outside it, which hands the command the token alone.
#
# The App is GitHub's answer to making one from a manifest, sealed with age
# to the machine's TPM, at ~/.local/share/github-broker/app.json.age, and
# the installation's id beside it in installation_id. Until both are there
# the broker answers that the App isn't made yet. It opens the App with
# the TPM identity at ~/.config/age/tpm.txt, which asks for no PIN.
#
# `github-broker` on PATH asks the socket and prints a token, for gh by
# hand: GH_TOKEN=$(github-broker) gh ... What follows it reaches the
# binary, so `github-broker --help` and `github-broker --json` answer too.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  socket = "%t/github-broker.sock";
  held = "${config.xdg.dataHome}/github-broker";
  asking = pkgs.writeShellScriptBin "github-broker" ''
    exec ${lib.getExe package} token --socket "$XDG_RUNTIME_DIR/github-broker.sock" "$@"
  '';
in
lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
  home.packages = [ asking ];

  systemd.user.sockets.github-broker = {
    Unit.Description = "Where a GitHub App's hour-long tokens are asked for";
    # Only the account may ask.
    Socket = {
      ListenStream = socket;
      SocketMode = "0600";
    };
    Install.WantedBy = [ "sockets.target" ];
  };

  systemd.user.services.github-broker = {
    Unit.Description = "Hands out a GitHub App's hour-long tokens";
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "serve"
        "--app"
        "${held}/app.json.age"
        "--installation"
        "${held}/installation_id"
        "--identity"
        "${config.xdg.configHome}/age/tpm.txt"
        "--socket"
        socket
      ];
      Slice = "background.slice";
      Restart = "on-failure";
      NoNewPrivileges = true;
    };
  };
}
