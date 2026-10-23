# Makes a passkey and a security key in a virtual machine's TPM through
# linux-id, with Yubico's FIDO library in a browser's place (passkey.py), and
# signs in with each. Each asks the account's askpass for a yes in linux-id's
# own words, a refusal makes nothing, a site that wants to know who's there
# is refused, and the keys outlive linux-id restarting, since the TPM holds
# them.
{ self, pkgs }:
let
  runtime = "/run/user/1000";
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

  testScript = ''
    import shlex

    def as_joel(command, succeed=True):
        environment = "export XDG_RUNTIME_DIR=${runtime}; cd ~; "
        run = machine.succeed if succeed else machine.fail
        return run("su - joel -c " + shlex.quote(environment + command))

    # What the askpass was asked since this was last read.
    def asked():
        return as_joel("cat ${runtime}/asked 2>/dev/null; rm -f ${runtime}/asked").splitlines()

    # The security key linux-id makes, as libfido2 finds it.
    def found():
        machine.wait_until_succeeds("su - joel -c 'fido2-token -L' | grep -q ^/dev/hidraw", timeout=60)

    machine.wait_for_unit("multi-user.target")
    machine.wait_for_file("${runtime}/bus")
    machine.wait_for_unit("default.target", "joel")

    with subtest("only the account at the seat may make the device"):
        found()
        t.assertIn("user:joel:rw-", machine.succeed("getfacl -p /dev/uhid"))

    with subtest("a refused yes makes no passkey"):
        as_joel("touch ${runtime}/refuse")
        as_joel("passkey make refused example.com passkey", succeed=False)
        t.assertEqual(asked(), ["confirm FIDO2 Register: example.com"])
        as_joel("rm ${runtime}/refuse")

    with subtest("a passkey is made in the TPM and signs in after a yes"):
        as_joel("passkey make passkey example.com passkey")
        t.assertEqual(asked(), ["confirm FIDO2 Register: example.com"])
        # The site keeps it on the key, so it signs in with no name given.
        as_joel("test -s ~/.config/linux-id/creds.json")
        # linux-id takes a yes as good for five seconds.
        machine.sleep(6)
        as_joel("passkey sign passkey example.com passkey")
        t.assertEqual(asked(), ["confirm FIDO2 Authenticate: example.com"])

    with subtest("a security key is added and signs in, asked without the site"):
        as_joel("passkey make security example.org security")
        as_joel("passkey sign security example.org security")
        t.assertEqual(asked(), ["confirm FIDO Confirm Register", "confirm FIDO Confirm Auth"])

    with subtest("a site that wants to know who's there is refused, unasked"):
        as_joel("passkey make verified example.net verified", succeed=False)
        t.assertEqual(asked(), [])

    with subtest("the keys outlive linux-id, since the TPM holds them"):
        as_joel("systemctl --user restart linux-id")
        # The old device is gone once udev has heard, so the next one found
        # is the new one's.
        machine.succeed("udevadm settle")
        found()
        as_joel("passkey sign passkey example.com passkey")
        as_joel("passkey sign security example.org security")
  '';
}
