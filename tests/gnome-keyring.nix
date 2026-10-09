# Stores a secret in a virtual machine's keyring the way Chrome and Slack do,
# then starts the keyring again, as the next login does, and reads the secret
# back. Nothing may ask for a password along the way: there's no one here to
# answer, so a prompt waits until the timeout fails the step. The keyring on
# disk must not hold the secret in the clear.
{ self, pkgs }:
let
  runtime = "/run/user/1000";
in
pkgs.testers.runNixOSTest {
  name = "gnome-keyring";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.gnome-keyring
    ];

    virtualisation.tpm.enable = true;

    users.users.joel.isNormalUser = true;
    home-manager.users.joel.home.stateVersion = "26.05";

    environment.systemPackages = [ pkgs.libsecret ];

    # Stands in for niri, whose unit pulls in the session's early services
    # the same way: graphical-session-pre.target refuses a direct start.
    systemd.user.services.compositor = {
      wants = [ "graphical-session-pre.target" ];
      after = [ "graphical-session-pre.target" ];
      serviceConfig.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
    };
  };

  testScript = ''
    import shlex

    def as_joel(command):
        environment = (
            "export XDG_RUNTIME_DIR=${runtime}"
            " DBUS_SESSION_BUS_ADDRESS=unix:path=${runtime}/bus; cd ~; "
        )
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    # The compositor starts only once the keyring is unlocked and ready.
    def log_in():
        as_joel("systemctl --user start compositor.service")
        as_joel("systemctl --user is-active gnome-keyring.service")

    def log_out():
        as_joel("systemctl --user stop compositor.service gnome-keyring.service")

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")

    with subtest("the keyring's password is sealed to the TPM at the first login"):
        log_in()
        # The header's first 16 bytes name the key: the TPM's and the host's,
        # scoped to one account. systemd falls back to the host's alone,
        # silently, when it can't reach the TPM.
        key = as_joel(
            "base64 -d ~/.local/state/gnome-keyring/login.cred | head -c 16 | od -An -tx1"
        )
        t.assertEqual(key.split(), "ef 4a c1 36 79 a9 48 0e a7 db 68 89 7f 9f 16 5d".split())

    with subtest("a secret is stored without a prompt, and not in the clear"):
        as_joel(
            "printf hunter2 | timeout 10"
            " secret-tool store --label=Slack service slack account joel"
        )
        as_joel("test -s ~/.local/share/keyrings/login.keyring")
        as_joel("! grep -q hunter2 ~/.local/share/keyrings/login.keyring")

    with subtest("the next login reads it back without a prompt"):
        log_out()
        log_in()
        secret = as_joel("timeout 10 secret-tool lookup service slack account joel")
        t.assertEqual(secret, "hunter2")
  '';
}
