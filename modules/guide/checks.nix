# The checks of guide, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  clippyConfig,
  clippyLints,
  holds,
  lib,
  nixpkgs,
  onLinux,
  passesClippy,
  pkgs,
  rust-overlay,
  self,
  ...
}:
onLinux {
  # The guide's shell holds the QEMU its VM tests boot under, so nix-direnv's
  # root for it keeps that QEMU, which no cache has, from being collected
  # and compiled again for an hour or more.
  the-guide-shell-keeps-its-vm-tests-qemu =
    holds "the-guide-shell-keeps-its-vm-tests-qemu"
      "the guide's development shell doesn't hold its VM tests' QEMU, so collecting the store compiles it again."
      (
        (self.devShells.${pkgs.stdenv.hostPlatform.system}.guide.GUIDE_VM_TESTS_QEMU or null)
        == "${(import ../../tests/vm/guide/canokeys.nix { inherit pkgs; }).qemu}"
      );

  # The stick types Colemak on the keyboard of every model core describes,
  # named here by hand so a model its own gathering misses fails: a
  # passphrase typed in the wrong layout locks its owner out of the disk.
  the-stick-types-colemak-on-every-built-in-keyboard =
    let
      stick = (bareNixos [ self.nixosModules.guide-stick ]).config.jitsusama.keyboard.builtIn;
      model =
        name:
        (nixpkgs.lib.nixosSystem {
          modules = [
            self.nixosModules.${name}
            ../keyboard/nixos.nix
          ];
        }).config.jitsusama.keyboard.builtIn;
      models = [
        "apple-macbook-air-m2-a2681"
        "apple-macbook-pro-m4-pro-a3401"
        "dell-xps-14-da14260"
        "microsoft-surface-laptop-4-1958"
      ];
      missed = lib.filter (id: !(lib.elem id stick)) (lib.concatMap model models);
    in
    holds "the-stick-types-colemak-on-every-built-in-keyboard"
      "The stick doesn't remap these built-in keyboards: ${lib.concatStringsSep ", " missed}."
      (missed == [ ]);

  # A stick asked to carry Apple's restore image lays it as a partition of
  # its own after the system's, named for the zip `guide revive` reads,
  # with the usbmuxd a revive starts; core's own stick carries none, so
  # nothing else pays for tens of gigabytes.
  a-stick-carries-apple-restores-apart =
    let
      restore = pkgs.callPackage ./apple-restore.nix { } {
        url = "https://updates.cdn-apple.com/2026FallFCS/0/UniversalMac_27.2_26B123_Restore.ipsw";
        hash = lib.fakeHash;
      };
      stick =
        restores:
        (bareNixos [
          self.nixosModules.guide-stick
          { jitsusama.stick.appleRestores = restores; }
        ]).config;
      carrying = stick [ restore ];
      plain = stick [ ];
      partition = carrying.image.repart.partitions."90-apple-restore-0".repartConfig or { };
      named = lib.attrNames carrying.image.repart.partitions;
      wrong =
        lib.optional (restore.name != "UniversalMac_27.2_26B123_Restore.erofs") "its image's name"
        ++ lib.optional (partition.Label or null != "UniversalMac_27.2_26B123") "its partition's name"
        ++ lib.optional (partition.CopyBlocks or null != "${restore}") "what fills it"
        ++ lib.optional (lib.last named != "90-apple-restore-0") "its place after the system"
        ++ lib.optional (!carrying.systemd.services ? guide-usbmuxd) "its usbmuxd"
        ++ lib.optional (
          !lib.hasInfix "guide revive" carrying.environment.loginShellInit
        ) "its console's word"
        ++ lib.optional (
          lib.any (lib.hasPrefix "90-apple-restore") (lib.attrNames plain.image.repart.partitions)
          || plain.systemd.services ? guide-usbmuxd
        ) "core's own stick carrying none";
    in
    holds "a-stick-carries-apple-restores-apart"
      "A stick carrying Apple's restore image gets these wrong: ${lib.concatStringsSep ", " wrong}."
      (wrong == [ ]);

  # The keys an account's modules name in the TPM reach what the installed
  # guide reads, each with its account and a comment saying whose and
  # where: tpm-signing's two and ssh-tpm-agent's login key, which
  # tpm-signing brings. Each says how `guide finish` adds it, titled by
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

  # A step a machine's repo adds once someone is signed in reaches what
  # `guide finish` reads, its check by the path of its program; one given
  # GitHub's id, or another's, is refused.
  the-guide-finishes-with-the-machines-own-steps =
    let
      check = pkgs.writeShellScriptBin "enrolled" "exit 0";
      step = id: {
        inherit id;
        title = "Enrol this machine with work";
        command = "enroll";
        about = "It signs you in through the browser.";
        done = check;
      };
      machine =
        steps:
        bareNixos [
          self.nixosModules.disko
          self.nixosModules.guide
          { jitsusama.guide.onceSignedIn = steps; }
        ];
      facts =
        # Read for what it says, the check's program's path among it.
        builtins.fromJSON (
          builtins.unsafeDiscardStringContext
            (machine [ (step "enrol") ]).config.environment.etc."guide/machine.json".text
        );
      refused =
        steps:
        lib.any (
          check: !check.assertion && lib.hasPrefix "Each of jitsusama.guide.onceSignedIn's" check.message
        ) (machine steps).config.assertions;
    in
    holds "the-guide-finishes-with-the-machines-own-steps"
      "`guide finish` doesn't see the steps a machine's repo adds, or two with one id aren't refused."
      (
        facts.once_signed_in == [
          {
            id = "enrol";
            title = "Enrol this machine with work";
            command = "enroll";
            about = "It signs you in through the browser.";
            done = "${check}/bin/enrolled";
          }
        ]
        && refused [ (step "github-keys") ]
        && refused [
          (step "enrol")
          (step "enrol")
        ]
        && !(refused [ (step "enrol") ])
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
