# Makes a passkey and a security key in a virtual machine's TPM through
# linux-id, with Yubico's FIDO library in a browser's place (passkey.py), and
# signs in with each. Each asks the account's askpass for a yes in linux-id's
# own words, a refusal makes nothing, a site that wants to know who's there
# is refused, and the keys outlive linux-id restarting, since the TPM holds
# them. The script is linux-id-vm-test.py.
{ self, pkgs }:
let
  runtime = "/run/user/1000";
  # What the script needs of the above, which it reads at its top.
  values = pkgs.writeText "linux-id-values.json" (builtins.toJSON { inherit runtime; });
  # Answers as Joel does through the shell's prompt, and writes down each
  # question, until it's told to refuse.
  askpass = pkgs.writeShellScript "askpass" ''
    echo "''${SSH_ASKPASS_PROMPT:-secret} $1" >> ${runtime}/asked
    [ ! -e ${runtime}/refuse ]
  '';
  # ruff and mypy read it through `nix fmt`, so the writers' flake8 is off.
  passkey = pkgs.writers.writePython3Bin "passkey" {
    libraries = [ pkgs.python3Packages.fido2 ];
    doCheck = false;
  } (builtins.readFile ./passkey.py);
in
pkgs.testers.runNixOSTest {
  name = "linux-id";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.linux-id
    ];

    virtualisation.tpm.enable = true;

    # Signed in at the seat, as Joel is at the desktop, which is what gives
    # the account the devices.
    users.users.joel.isNormalUser = true;
    services.getty.autologinUser = "joel";
    home-manager.users.joel = {
      systemd.user.sessionVariables.SSH_ASKPASS = "${askpass}";
      home.stateVersion = "26.05";
    };
    environment.systemPackages = [
      passkey
      pkgs.libfido2
    ];
  };

  # The script reads the values from the file put in place of its marker.
  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./linux-id-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
