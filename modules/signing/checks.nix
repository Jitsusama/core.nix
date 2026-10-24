# The checks of signing and the two kinds of hardware it signs with, which
# checks.nix gathers with every other folder's; its opening comment says
# what each kind of check proves. vm-commits-are-signed signs with a TPM's
# keys in a machine; a Mac's Secure Enclave can only be reached on a Mac, so
# its half is proven here by what a Mac's account is given.
{
  account,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # What git is told about signing in an account, from the include signing
  # writes last.
  signing =
    home:
    (lib.findFirst (include: include ? contents.gpg.format) {
      contents = { };
    } home.programs.git.includes).contents;
in
{
  # Each machine signs with its own hardware through the same program: a
  # Linux workstation through the TPM's agent, a Mac through its Secure
  # Enclave with the Mac's own ssh-keygen, and with no TPM agent or systemd
  # unit of Linux's. Both sign every commit and tag with Joel's key by
  # default, and a Mac has se-keys to make its keys.
  each-machine-signs-with-its-hardware =
    let
      linux = account workstation;
      mac = account macWorkstation;
      # What a layer that only lists the signers is given, with no hardware's
      # module to say how to sign.
      listing = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.signing
            { jitsusama.signing.allowedSigners = [ "someone@example.com ssh-ed25519 AAAA" ]; }
          ];
        }
      ]);
      onLinux = signing linux;
      onMac = signing mac;
      macPackages = map (package: package.name or "") mac.home.packages;
    in
    holds "each-machine-signs-with-its-hardware"
      "A machine signs without its hardware's program or key, a Mac is given Linux's TPM agent, a Mac has no se-keys to make its keys, or signing alone, with no hardware, signs or fails."
      (
        lib.hasSuffix "/bin/tpm-sign" onLinux.gpg.ssh.program
        && onLinux.user.signingKey == "/home/someone/.local/share/signing/joel-signing.pub"
        && linux.systemd.user.services ? signing-key
        && lib.hasSuffix "/bin/se-sign" onMac.gpg.ssh.program
        && onMac.user.signingKey == "/Users/someone/.ssh/joel-signing"
        && mac.jitsusama.signing.signer.sshKeygen == "/usr/bin/ssh-keygen"
        && mac.jitsusama.signing.signer.environment ? SSH_SK_PROVIDER
        && !(mac.services.ssh-tpm-agent.enable or false)
        && !(mac.systemd.user.services ? signing-key)
        && builtins.elem "se-keys" macPackages
        && signing listing == { }
        && !(listing.xdg.configFile ? "git/allowed_signers")
        && builtins.all (on: on.commit.gpgSign && on.tag.gpgSign && on.gpg.format == "ssh") [
          onLinux
          onMac
        ]
      );
}
// onLinux {
  # The program picks whose key signs and takes back a signature that
  # doesn't verify, run against a stand-in ssh-keygen
  # (tests/signing-program.sh says each case).
  the-signer-picks-who-signs =
    pkgs.runCommandLocal "the-signer-picks-who-signs"
      {
        nativeBuildInputs = [
          pkgs.openssh
          pkgs.util-linux
        ];
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/signing-program.sh} ${./sign.sh} \
          ${../../tests/empty-signature.sig}
        touch $out
      '';
}
