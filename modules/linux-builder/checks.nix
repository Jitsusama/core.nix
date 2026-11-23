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
  # A line of the VM's launchd script that starts it under caffeinate -s.
  underCaffeinate = " */usr/bin/caffeinate -s /nix/store/[^ ]+/bin/create-builder";

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

  # A changed guest's network serves it no DNS, so macOS's Internet Sharing
  # leaves the Mac's port 53 to its own resolvers: with it, on 15 November
  # the Mac's VPN couldn't bind its resolver and dropped for hours. vzvm is
  # patched to put such a guest on a NAT without the DNS proxy, and the
  # guest asks resolvers of its own. One given nothing keeps the cached
  # guest, whose network is nixpkgs' own, so it still needs no build.
  linux-builder-leaves-the-macs-port-53-alone =
    holds "linux-builder-leaves-the-macs-port-53-alone"
      "A changed guest's network still serves it DNS from the Mac, its vzvm isn't the one patched to leave it out, it has no resolvers of its own, or a guest given nothing changed its network."
      (
        opened.virtualisation.vz.extraConfig.dnsProxy or true == false
        && lib.elem "vzvm-without-dns-proxy.patch" (
          map (patch: baseNameOf (toString patch)) (opened.virtualisation.vz.package.patches or [ ])
        )
        && opened.networking.nameservers != [ ]
        && lib.hasInfix "nooption domain_name_servers" opened.networking.dhcpcd.extraConfig
        && plain.virtualisation.vz.extraConfig == cached.virtualisation.vz.extraConfig
      );

  # A changed guest's network is the one set, never one vmnet picks: on 21
  # November vmnet put it inside a home network's /22, and mDNSResponder
  # renamed the Mac twice and spun a core. The prefix becomes vmnet's mask
  # octet by octet, so a /20 is tried as well as the default /24, and vmnet
  # is given the network's first address, the Mac's, since given the
  # network's own it gave the guest no network on 22 November; a /30 ending
  # in 252, the widest last octet a network can have, proves it's the next
  # address and not .1 written in. It pins what vmnet is handed, not what
  # vmnet makes of it, which only a Mac booting the guest shows. Another
  # network changes the daemon's command, so a switch loads the VM again,
  # even one booted out by hand. A network that isn't private, has an octet
  # past 255 or a host bit set is refused by the Mac's configuration, rather
  # than looping at start, and one set on a guest that keeps macOS's NAT is
  # only warned of, since it's never used. The patch reads the keys the
  # module writes. One given nothing keeps the cached guest's network, so it
  # still needs no build.
  linux-builder-keeps-its-network-where-it-is-set =
    let
      narrowly = {
        authorizedKeys = [ another ];
        network = "172.31.240.0/20";
      };
      narrow = guestOf narrowly;
      tightest = guestOf {
        authorizedKeys = [ another ];
        network = "172.31.250.252/30";
      };
      started = settings: (macOf settings).launchd.daemons.linux-builder-vz.script;
      # Whether the Mac's configuration refuses a network.
      refusedBy =
        settings:
        lib.any (
          said: !said.assertion && lib.hasInfix "linux-builder.network" said.message
        ) (macOf settings).assertions;
      refuses =
        network:
        refusedBy {
          authorizedKeys = [ another ];
          inherit network;
        };
      warned = settings: lib.any (lib.hasInfix "linux-builder.network") (macOf settings).warnings;
      patch = builtins.readFile ./vzvm-own-subnet.patch;
      written = opened.virtualisation.vz.extraConfig;
      read = key: lib.hasInfix "forKey: .${key})" patch && written ? ${key};
    in
    holds "linux-builder-keeps-its-network-where-it-is-set"
      "A changed guest's network isn't the one set, by default 192.168.239.0/24, vmnet isn't given its first address, or a /30's isn't the one after its own, a /20 isn't given vmnet as 255.255.240.0, a public network, an octet past 255 or a host bit set isn't refused, the patch never gives vmnet the subnet, a network set on an unchanged guest isn't warned of or is refused, the patch doesn't read the keys written, its vzvm isn't the one patched, another network leaves the daemon's command as it was, or a guest given nothing changed its network."
      (
        opened.virtualisation.vz.extraConfig.ipv4Subnet or null == "192.168.239.1"
        && opened.virtualisation.vz.extraConfig.ipv4Mask or null == "255.255.255.0"
        && !refuses "192.168.239.0/24"
        && !refuses "172.31.240.0/20"
        && refuses "192.168.300.0/24"
        && refuses "192.168.239.7/24"
        && refuses "172.31.241.0/20"
        && refuses "8.8.8.0/24"
        && refuses "127.0.0.0/24"
        && refuses "172.32.0.0/16"
        && refuses "172.15.0.0/16"
        && refuses "192.169.0.0/16"
        && !refuses "172.16.0.0/16"
        && !refuses "10.211.0.0/16"
        && !refusedBy { network = "8.8.8.0/24"; }
        && lib.hasInfix "set_ipv4_subnet(shared, &address, &netmask)" patch
        && warned { network = "172.31.240.0/20"; }
        && !warned { }
        && !warned narrowly
        && read "ipv4Subnet"
        && read "ipv4Mask"
        && narrow.virtualisation.vz.extraConfig.ipv4Subnet or null == "172.31.240.1"
        && narrow.virtualisation.vz.extraConfig.ipv4Mask or null == "255.255.240.0"
        && tightest.virtualisation.vz.extraConfig.ipv4Subnet or null == "172.31.250.253"
        && tightest.virtualisation.vz.extraConfig.ipv4Mask or null == "255.255.255.252"
        && lib.elem "vzvm-own-subnet.patch" (
          map (patch: baseNameOf (toString patch)) (opened.virtualisation.vz.package.patches or [ ])
        )
        && plain.virtualisation.vz.extraConfig == cached.virtualisation.vz.extraConfig
        && started { authorizedKeys = [ another ]; } != started narrowly
      );

  # A changed guest's vzvm makes its network once and gives it to every boot:
  # on 23 November the boot after an idle stop asked vmnet for a second on
  # the subnet the first still held, was refused, and failed the build that
  # woke it. The module's patches are applied in its order to the source the
  # Mac builds, with no fuzz, so one that no longer applies fails here, not
  # only on a Mac; then the reuse must come before any network is made.
  # Whether vmnet takes the reused network is what only a Mac shows.
  linux-builder-boots-again-on-its-own-network =
    let
      patches = opened.virtualisation.vz.package.patches or [ ];
    in
    pkgs.runCommandLocal "linux-builder-boots-again-on-its-own-network" { } ''
      cp -r ${pkgs.vzvm.src}/vzvm source
      chmod -R u+w source
      cd source
      for patch in ${lib.concatMapStringsSep " " (patch: "${patch}") patches}; do
        patch -p1 -F0 --no-backup-if-mismatch --quiet < "$patch"
      done
      file=Sources/vzvm/VirtualMachineBuilder.swift
      reuse='return VZVmnetNetworkDeviceAttachment(network: network)'
      # grep finding nothing is an answer here, not an error to stop on.
      reused=$(grep -n "$reuse" $file | cut -d: -f1 || true)
      made=$(grep -n 'vmnet_network_create(' $file | cut -d: -f1 || true)
      if [ -z "$reused" ] || [ -z "$made" ] || [ "$(printf '%s\n' "$made" | wc -l)" != 1 ] \
        || [ "$reused" -ge "$made" ]; then
        echo "A later boot isn't given the network the process made (reused at line" \
          "''${reused:-none}, made at ''${made:-none}), so vmnet is asked for another." >&2
        exit 1
      fi
      touch $out
    '';

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

  # The VM runs under caffeinate -s, so the Mac stays awake while it serves
  # and its builds don't stop with a Mac that sleeps.
  linux-builder-keeps-a-serving-mac-awake =
    holds "linux-builder-keeps-a-serving-mac-awake"
      "The VM doesn't run under caffeinate -s, so the Mac could sleep while it serves."
      (
        builtins.any (line: builtins.match underCaffeinate line != null) (
          lib.splitString "\n" (macOf { }).launchd.daemons.linux-builder-vz.script
        )
      );
}
