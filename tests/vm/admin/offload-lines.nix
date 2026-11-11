# A machine whose root reads its builders from nix-offload's lines rather
# than from the lender's own files: the service starts with its settings as
# its own account, reads the token the lender's account leaves it, asks the
# cloud (a stand-in, offload-lines-gcloud.sh) whether the lent instance is
# as its maker made it and for its host key, and passes root only the
# lines naming a builder it trusts, in a file the lender's account can't
# write. The tests are offload-lines-vm-test.py,
# which pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  # The host key the cloud gives for the lent instance, and a standing
  # builder's, as their lines carry them.
  lentKey = "AAAAC3NzaC1lZDI1NTE5AAAAILentLentLentLentLentLentLentLentLentLentLen";
  standingKey = "c3NoLWVkMjU1MTkgQUFBQUMzTnphQzFsWkRJMU5URTVBQUFBSVN0YW5kaW5n";
  lentUri = "ssh-ng://nix-offload?compress=true";
  standingUri = "ssh-ng://builder@nearby";
  gcloud = pkgs.writeShellApplication {
    name = "gcloud";
    runtimeInputs = [ pkgs.jq ];
    text = builtins.readFile ./offload-lines-gcloud.sh;
  };
  # What the tests need from Nix, in one file, their `values` fixture.
  values = pkgs.writeText "offload-lines-values.json" (
    builtins.toJSON {
      inherit
        lentKey
        standingKey
        lentUri
        standingUri
        ;
    }
  );
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "offload-lines";

  nodes.machine = {
    imports = [ self.nixosModules.nix-offload ];
    users.users.someone = {
      isNormalUser = true;
      group = "users";
    };
    environment.etc."offload-lines/host-key".text = "ssh-ed25519 ${lentKey}\n";
    jitsusama.nix-offload = {
      account = "someone";
      lines = {
        project = "a-lab";
        prefix = "a-builder-";
        zones = [ "a-zone" ];
        startupScript = pkgs.writeText "startup-script" "echo a builder\n";
        images = [ "https://compute.example/images/a-" ];
        lent = [
          {
            uri = lentUri;
            folder = "/run/nix-offload";
          }
        ];
        standing = [
          {
            uri = standingUri;
            key = "/etc/nix/builder_ed25519";
            hostKey = standingKey;
          }
        ];
        inherit gcloud;
      };
    };
  };

  tests = ./offload-lines-vm-test.py;
  inherit values;
}
