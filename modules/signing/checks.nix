# The checks of signing and the kinds of hardware it signs with, which
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
  # unit of Linux's, and a Linux workstation with no TPM with Joel's key on
  # his YubiKey and the agents' key in a file, served on the socket the
  # TPM's would be. All sign every commit and tag with Joel's key by
  # default, and the two with no TPM have a program to make their keys.
  each-machine-signs-with-its-hardware =
    let
      linux = account workstation;
      mac = account macWorkstation;
      noTpm = account (bareNixos [
        self.nixosModules.workstation
        { jitsusama.firmware.tpm = false; }
      ]);
      onNoTpm = signing noTpm;
      noTpmPackages = map (package: package.name or "") noTpm.home.packages;
      # What a machine repository's own modules that only list the signers
      # are given, with no hardware's module to say how to sign.
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
      "A machine signs without its hardware's program or key, a Mac or a Linux machine with no TPM is given the TPM's agent, either has nothing to make its keys, the agents' key isn't served where there's no TPM, or signing alone, with no hardware, signs or fails."
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
        && lib.hasSuffix "/bin/yubikey-sign" onNoTpm.gpg.ssh.program
        && onNoTpm.user.signingKey == "/home/someone/.local/share/signing/joel-signing"
        && !(noTpm.services.ssh-tpm-agent.enable or false)
        && !(noTpm.systemd.user.services ? signing-key)
        && noTpm.systemd.user.sockets.agent-signing.Socket.ListenStream == "%t/agent-signing.sock"
        && builtins.elem "yubikey-keys" noTpmPackages
        && noTpm.jitsusama.yubikeySpare
        && signing listing == { }
        && !(listing.xdg.configFile ? "git/allowed_signers")
        && builtins.all (on: on.commit.gpgSign && on.tag.gpgSign && on.gpg.format == "ssh") [
          onLinux
          onMac
          onNoTpm
        ]
      );
}
// onLinux {
  # se-keys, the Mac's key maker, builds, shellcheck and all, for a name with
  # an apostrophe, which the agents' key's name always has. It's only a
  # script, so it builds on Linux, where a Mac's whole system can't.
  se-keys-builds-for-any-name =
    let
      mac = account (bareNixos [
        {
          home-manager.sharedModules = [
            self.homeModules.secure-enclave-signing
            { jitsusama.identity.name = "Someone O'Neill"; }
          ];
        }
      ]);
    in
    lib.findFirst (package: (package.name or "") == "se-keys") (throw "no se-keys") mac.home.packages;

  # yubikey-keys, the key maker where there's no TPM, builds, shellcheck and
  # all.
  yubikey-keys-builds =
    lib.findFirst (package: (package.name or "") == "yubikey-keys") (throw "no yubikey-keys")
      (account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.yubikey-signing ]; } ]))
      .home.packages;

  # The two key makers answer --help and refuse a word, before they make a
  # key or ask the YubiKey for a touch: run with an empty home, nothing
  # may appear in it. se-keys refuses before it reaches the Mac's tools,
  # so a Linux build of it shows the same.
  key-makers-asked-wrongly-make-nothing =
    let
      maker =
        modules: name:
        lib.getExe (
          lib.findFirst (package: (package.name or "") == name) (throw "no ${name}")
            (account (bareNixos [ { home-manager.sharedModules = modules; } ])).home.packages
        );
      makers = [
        (maker [ self.homeModules.yubikey-signing ] "yubikey-keys")
        (maker [
          self.homeModules.secure-enclave-signing
          { jitsusama.identity.name = "Someone O'Neill"; }
        ] "se-keys")
      ];
    in
    pkgs.runCommandLocal "key-makers-asked-wrongly-make-nothing" { } ''
      ${lib.getExe pkgs.bash} ${./tests/key-makers-asked-wrongly.sh} "$PWD/home" \
        ${lib.escapeShellArgs makers}
      touch $out
    '';

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
        ${lib.getExe pkgs.bash} ${./tests/signing-program.sh} ${./sign.sh} \
          ${./tests/empty-signature.sig}
        touch $out
      '';

  # The agents that serve the agents' and nodes' keys sit with the agents
  # rather than among Joel's programs, where their processor counted as his,
  # and ask oomd to spare them there. With a TPM or without. Joel's own
  # login key's agent stays with his programs: it is his.
  the-signers-sit-with-the-agents =
    let
      linked =
        tpm:
        account (bareNixos [
          self.nixosModules.workstation
          {
            jitsusama.firmware.tpm = tpm;
            home-manager.sharedModules = [ { jitsusama.linkKeys = [ "personal" ]; } ];
          }
        ]);
      service = home: name: home.systemd.user.services.${name}.Service;
      sliceOf = home: name: (service home name).Slice or null;
      spared = home: name: ((service home name).ManagedOOMPreference or null) == "avoid";
      withAgents = home: lib.all (name: sliceOf home name == "agents.slice" && spared home name);
    in
    holds "the-signers-sit-with-the-agents"
      "An agent serving the agents' key or a realm's link key sits outside agents.slice or isn't spared by oomd there, or Joel's login key's agent sits with the agents."
      (
        sliceOf (linked true) "ssh-tpm-agent" == null
        && withAgents (linked true) [
          "agent-signing"
          "personal-link"
        ]
        && withAgents (linked false) [
          "agent-signing"
          "agent-signing-agent"
          "personal-link"
          "personal-link-agent"
        ]
      );
}
