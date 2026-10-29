# The checks of guide, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  clippyConfig,
  clippyLints,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  rust-overlay,
  self,
  ...
}:
onLinux {
  # The keys an account's modules name in the TPM reach what the installed
  # guide reads, each with its account and a comment saying whose and
  # where: tpm-signing's two and ssh-tpm-agent's login key, which
  # tpm-signing brings. Each says how `guide github` adds it, titled by
  # the machine.
  the-guide-makes-the-keys-the-modules-name =
    let
      machine = bareNixos [
        self.nixosModules.disko
        self.nixosModules.guide
        {
          networking.hostName = "laptop";
          home-manager.users.someone.imports = [ self.homeModules.tpm-signing ];
        }
      ];
      home = machine.config.home-manager.users.someone.home.homeDirectory;
      facts = builtins.fromJSON machine.config.environment.etc."guide/machine.json".text;
      key = file: comment: pin: kind: title: {
        account = "someone";
        file = "${home}/${file}";
        inherit comment pin;
        github = { inherit kind title; };
      };
    in
    holds "the-guide-makes-the-keys-the-modules-name"
      "The installed guide doesn't see the keys in the TPM that an account's modules name."
      (
        lib.sort (one: other: one.file < other.file) facts.keys == [
          (key ".local/share/signing/joel-signing" "someone@laptop signing" true "signing" "laptop signing")
          (key ".ssh/agent-signing" "someone@laptop agents" false "signing" "laptop agents")
          (key ".ssh/id_ecdsa" "someone@laptop" true "authentication" "laptop TPM")
        ]
      );

  # An account with ssh-tpm-agent keeps its login key's spare on each
  # YubiKey, which reaches what the installed guide reads; a second
  # account wanting one is refused, since a YubiKey holds one.
  the-guide-keeps-one-accounts-spare-on-the-yubikeys =
    let
      machine =
        others:
        bareNixos [
          self.nixosModules.disko
          self.nixosModules.guide
          {
            home-manager.users = {
              someone.imports = [ self.homeModules.ssh-tpm-agent ];
            }
            // lib.genAttrs others (_: {
              imports = [ self.homeModules.ssh-tpm-agent ];
              home.stateVersion = "26.05";
            });
            users.users = lib.genAttrs others (_: {
              isNormalUser = true;
            });
          }
        ];
      alone = machine [ ];
      facts = builtins.fromJSON alone.config.environment.etc."guide/machine.json".text;
      refused = lib.filter (check: !check.assertion) (machine [ "another" ]).config.assertions;
    in
    holds "the-guide-keeps-one-accounts-spare-on-the-yubikeys"
      "The installed guide doesn't see the account keeping a spare on the YubiKeys, or two accounts aren't refused."
      (
        facts.spares == [
          {
            account = "someone";
            folder = "${alone.config.home-manager.users.someone.home.homeDirectory}/.ssh";
            comment = "someone@yubikey";
          }
        ]
        && lib.any (check: lib.hasPrefix "Only one account" check.message) refused
      );

  # The guide's tests run as it builds: each step proven by observing it
  # again, each parser reading what real machines printed, each stick it
  # writes read back by sfdisk and fsck.fat, and each signature compared
  # with sbsign's. Its install check then finds every program it runs at
  # the path it was built with.
  guide-proves-its-steps = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the guide, its tests included.
  guide-passes-clippy =
    (passesClippy "guide-passes-clippy" (pkgs.callPackage ./package.nix { })).overrideAttrs
      { doInstallCheck = false; };

  # A Mac writes sticks too, through its own raw disks and cache, and no
  # check runs on a Mac, so clippy reads the guide as macOS would compile it.
  guide-compiles-for-macos =
    let
      toolchain = (pkgs.extend rust-overlay.overlays.default).rust-bin.stable.latest.minimal.override {
        targets = [ "aarch64-apple-darwin" ];
        extensions = [ "clippy" ];
      };
    in
    (pkgs.callPackage ./package.nix { }).overrideAttrs {
      pname = "guide-compiles-for-macos";
      CLIPPY_CONF_DIR = clippyConfig;
      buildPhase = ''
        export PATH=${toolchain}/bin:$PATH
        cargo clippy --target aarch64-apple-darwin --all-targets --offline -- \
          ${lib.escapeShellArgs clippyLints}
      '';
      doCheck = false;
      doInstallCheck = false;
      installPhase = ''
        touch $out
      '';
    };
}
