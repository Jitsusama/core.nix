# Stores a secret in a virtual machine's keyring the way Chrome and Slack do,
# then starts the keyring again, as the next login does, and reads the secret
# back, once after the session starts it and once when a program asks first.
# Nothing may ask for a password along the way: there's no one here to
# answer, so a prompt waits until the timeout fails the step. The keyring on
# disk must not hold the secret in the clear. The script is
# gnome-keyring-vm-test.py.
{ self, pkgs }:
let
  runtime = "/run/user/1000";
  # What the tests need of the above, as their values fixture.
  values = pkgs.writeText "gnome-keyring-values.json" (builtins.toJSON { inherit runtime; });
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "gnome-keyring";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.gnome-keyring
    ];

    virtualisation.tpm.enable = true;

    users.users.joel = {
      isNormalUser = true;
      # For a sign-in at the console, through PAM as the greeter's goes.
      initialPassword = "hunter3";
    };
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

  tests = ./gnome-keyring-vm-test.py;
  inherit values;
}
