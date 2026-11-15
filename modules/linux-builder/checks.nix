# The checks of the Linux builder, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareDarwin,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
let
  # A Mac running the builder with these settings, and the VM's guest on it.
  macOf =
    settings:
    (bareDarwin [
      self.darwinModules.linux-builder
      { jitsusama.linux-builder = settings; }
    ]).config;
  guestOf = settings: (macOf settings).jitsusama.linux-builder.builder.nixosConfig;
  # The features the Mac's line offers.
  offered =
    settings:
    lib.splitString "," (
      builtins.elemAt (lib.splitString " "
        (macOf settings).environment.etc."nix/linux-builder.conf".text
      ) 7
    );

  # The guest nixpkgs builds, which is the one its cache holds.
  cached = (bareDarwin [ ]).pkgs.darwin.linux-builder-vz.nixosConfig;
  # The features nixpkgs's own guest adds to its Nix's.
  nixpkgsOwn = cached.nix.settings.extra-system-features or [ ];

  another = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGRkdxraohorYagcirm3lEonh6at0F7DvFoLuEqBQajh elsewhere";
  plain = guestOf { };
  opened = guestOf { authorizedKeys = [ another ]; };
  collecting = guestOf { collectableBy = [ another ]; };
  # What the guest's SSH lets in as its builder, a key a line.
  letIn =
    guest:
    lib.filter (line: line != "") (
      lib.splitString "\n" guest.environment.etc."ssh/authorized_keys.d/builder".source.text
    );
  marked = guestOf { features = [ "own-code" ]; };
  system = guest: builtins.unsafeDiscardStringContext guest.system.build.toplevel.drvPath;

  # The lines of what sudo reads on a Mac with these settings that let
  # anything run without a password, and the rule each account given should
  # get: the two restarts, whole, and nothing else. nix-darwin writes the
  # file whatever this module says, with lines of its own.
  passwordless =
    settings:
    builtins.filter (lib.hasInfix "NOPASSWD") (
      lib.splitString "\n" (macOf settings).environment.etc."sudoers.d/10-nix-darwin-extra-config".text
    );
  rule =
    account:
    "${account} ALL=(root) NOPASSWD: "
    + "/bin/launchctl kickstart -k system/org.nixos.linux-builder-vz, "
    + "/bin/launchctl kickstart -k system/systems.determinate.nix-daemon";
in
{
  # A key from another machine lets it in to the VM's builder, where the
  # guest's SSH reads it. Without one, the guest is nixpkgs' own, sized, so
  # it still comes from the cache.
  linux-builder-lets-in-only-the-keys-it-is-given =
    holds "linux-builder-lets-in-only-the-keys-it-is-given"
      "The VM's builder didn't take exactly the keys it was given, or a VM given none isn't the cached one."
      (
        lib.hasInfix another opened.environment.etc."ssh/authorized_keys.d/builder".source.text
        && builtins.elem "/etc/ssh/authorized_keys.d/%u" opened.services.openssh.authorizedKeysFiles
        && builtins.length opened.services.openssh.authorizedKeysFiles == 2
        && plain.services.openssh.authorizedKeysFiles == cached.services.openssh.authorizedKeysFiles
        && system plain == system cached
        && system opened != system cached
      );

  # A collecting key lets its holder in to the VM's builder only to run the
  # collection, with no shell, pty or forwarding, and never as a builder, so
  # a machine whose agents hold it can free the VM's store without sudo on
  # the Mac. The guest is changed, so it keeps room as one given keys does.
  linux-builder-lets-collecting-keys-only-collect =
    holds "linux-builder-lets-collecting-keys-only-collect"
      "The VM let a collecting key in other than restricted to collect.sh, let it in plainly too, or didn't read the file it's written to."
      (
        let
          lines = letIn collecting;
          forced = lib.filter (lib.hasPrefix "restrict,command=\"") lines;
          files = collecting.services.openssh.authorizedKeysFiles;
        in
        lines == forced
        && builtins.length forced == 1
        && lib.hasSuffix "/bin/collect\" ${another}" (builtins.head forced)
        && builtins.elem "/etc/ssh/authorized_keys.d/%u" files
        && collecting.nix.settings.min-free != cached.nix.settings.min-free
      );

  # The collection does what each ask says and prints the store's free
  # bytes; anything else is refused before it runs a thing.
  linux-builder-collects-only-what-it-is-asked =
    pkgs.runCommandLocal "linux-builder-collects-only-what-it-is-asked" { }
      ''
        $BASH ${./collect-test.sh} ${./collect.sh}
        touch $out
      '';

  # A guest already changed asks chrony for the time each minute and steps
  # any error past a second, since its clock stops while the Mac sleeps; one
  # given nothing keeps the cached guest's clock, so it still needs no build.
  linux-builder-keeps-a-changed-guests-time =
    holds "linux-builder-keeps-a-changed-guests-time"
      "A changed guest doesn't step its clock with chrony at any time, or a guest given nothing does."
      (
        opened.services.chrony.enable
        && !opened.services.timesyncd.enable
        && lib.hasInfix "makestep 1 -1" opened.services.chrony.extraConfig
        && marked.services.chrony.enable
        && !plain.services.chrony.enable
      );

  # A changed guest collects garbage once a sixth of its disk is free, back
  # to five twelfths, so a build has room: nixpkgs' 1 GiB and 3 GiB left a
  # 60 GiB store 90 to 93% full on 9 November, with builds failing for disk.
  # One given nothing keeps the cached guest's, so it still needs no build.
  linux-builder-keeps-room-in-a-changed-guest =
    holds "linux-builder-keeps-room-in-a-changed-guest"
      "A changed guest doesn't collect garbage from a sixth of its disk free to five twelfths, or a guest given nothing does."
      (
        let
          gib = 1024 * 1024 * 1024;
          sized = guestOf {
            disk = 60 * 1024;
            features = [ "own-code" ];
          };
        in
        sized.nix.settings.min-free == 10 * gib
        && sized.nix.settings.max-free == 25 * gib
        && plain.nix.settings.min-free == cached.nix.settings.min-free
        && plain.nix.settings.max-free == cached.nix.settings.max-free
      );

  # Only the accounts named may restart the VM and the Nix daemon without a
  # password, each by the whole command, so the rule lets nothing else
  # through; with none named, sudo gets no rule at all.
  linux-builder-lets-only-those-named-restart-it =
    holds "linux-builder-lets-only-those-named-restart-it"
      "The Mac's sudo rule for restarting the Linux builder wasn't exactly the two restarts for each account named, or there was one with none named."
      (
        passwordless { } == [ ]
        &&
          passwordless {
            restartableBy = [
              "someone"
              "other"
            ];
          } == [
            (rule "someone")
            (rule "other")
          ]
      );

  # A feature of the machine's own is offered on the Mac's line and by the
  # guest's Nix, beside the builder's own, so a piece asking for it is both
  # sent there and built there; with none, both offer only the builder's.
  # The guest's own are nixpkgs's, the cached guest's, which since 26.11
  # include what its nspawn tests need; the Mac's line leaves those out.
  linux-builder-offers-the-features-it-is-given =
    holds "linux-builder-offers-the-features-it-is-given"
      "The VM's builder didn't offer exactly benchmark, big-parallel and the features it was given, on the Mac's line and in the guest's Nix."
      (
        offered { } == [
          "benchmark"
          "big-parallel"
        ]
        &&
          offered { features = [ "own-code" ]; } == [
            "benchmark"
            "big-parallel"
            "own-code"
          ]
        && lib.subtractLists nixpkgsOwn marked.nix.settings.extra-system-features == [ "own-code" ]
        && (plain.nix.settings.extra-system-features or [ ]) == nixpkgsOwn
        && builtins.elem "big-parallel" marked.nix.settings.system-features
      );
}
