# The checks of shaper, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  bare = bareNixos [ self.nixosModules.shaper ];
  services = bare.config.systemd.services;
  named =
    (bareNixos [
      self.nixosModules.shaper
      { jitsusama.shaper.controller = "one-way"; }
    ]).config.systemd.services;
  # A machine whose two networks each name a house, one reporting over
  # IPv4 and one over IPv6 from either of two addresses.
  housed =
    (bareNixos [
      self.nixosModules.shaper
      {
        jitsusama.shaper.networks =
          lib.mapAttrs
            (_: address: {
              up = 12000;
              house = {
                most = 7000;
                from = address;
              };
            })
            {
              "00:00:5e:00:53:01" = "192.0.2.7";
              "00:00:5e:00:53:02" = [
                "2001:db8::7"
                "2001:db8::8"
              ];
            };
      }
    ]).config.networking.firewall;
  # The port src/house.rs listens on, read from it so the two can't part.
  port = lib.pipe ./src/house.rs [
    builtins.readFile
    (lib.splitString "\n")
    (map (builtins.match "pub const PORT: u16 = ([0-9_]+);"))
    (lib.findFirst (found: found != null) (
      throw "shaper-hears-only-the-house: no `pub const PORT: u16` line in src/house.rs"
    ))
    lib.head
    (lib.replaceStrings [ "_" ] [ "" ])
  ];
  # The machine's input rules, which must be the houses' three addresses
  # alone, and no port, range or interface left open to all that takes the
  # port in.
  rules = lib.filter (line: line != "") (lib.splitString "\n" housed.extraInputRules);
  houseRules =
    rules == [
      "ip saddr 192.0.2.7 udp dport ${port} accept"
      "ip6 saddr 2001:db8::7 udp dport ${port} accept"
      "ip6 saddr 2001:db8::8 udp dport ${port} accept"
    ];
  takesThePort =
    open:
    lib.elem (lib.toInt port) open.allowedUDPPorts
    || lib.any (
      range: range.from <= lib.toInt port && lib.toInt port <= range.to
    ) open.allowedUDPPortRanges;
  opened =
    houseRules
    && !lib.any takesThePort ([ housed ] ++ lib.attrValues housed.interfaces)
    # Loopback is trusted on every machine, and reaches no other.
    && lib.subtractLists [ "lo" ] housed.trustedInterfaces == [ ];
  # The check a house's address is held to, read from the module's own
  # option, against addresses Rust's parse takes and refuses alike, and
  # IPv4 ones written as IPv6, in each spelling.
  address =
    (
      (bare.options.jitsusama.shaper.networks.type.getSubOptions [ ])
      .house.type.nestedTypes.elemType.getSubOptions
        [ ]
    ).from.type;
  taken = [
    "192.0.2.7"
    "10.0.0.255"
    "2001:db8::7"
    "fe80::1"
    "1:2:3:4:5:6:7:8"
    "1::ffff:a:b"
  ];
  refused = [
    "the mac"
    "1.2.3"
    "10.0.0.256"
    "192.168.001.007"
    "1::2::3"
    "1:2:3:4:5:6:7:8:9"
    "1:2:3:4:5:6:7::8"
    "12345::1"
    "::ffff:192.0.2.7"
    "::ffff:c000:207"
    "::FFFF:C000:207"
    "0:0:0:0:0:ffff:c000:207"
    "0:0:0:0:0:ffff:c000::"
    "::ffff:0:207"
    "::c000:207"
    "0:0:0:0:0:0:c000:207"
    "::"
  ];
  # Whether the option takes `value`, as a module setting it would: a
  # list's addresses are each checked only as the list is merged.
  takes =
    value:
    (builtins.tryEval (
      builtins.deepSeq
        (lib.evalModules {
          modules = [
            {
              options.from = lib.mkOption { type = address; };
              config.from = value;
            }
          ];
        }).config.from
        true
    )).success;
  # A list is held to the same check in each of its addresses; one of
  # none is the assertions' to judge, by whether its house names a key.
  misread =
    lib.filter (text: !address.check text) taken
    ++ lib.filter address.check refused
    ++ lib.optional (!takes taken) "the list of every address taken"
    ++ lib.optional (takes [
      "192.0.2.7"
      "the mac"
    ]) "a list holding a name";
  # What the shaper's assertions refuse of a machine whose networks name
  # `houses`, by network: their messages, the machine's others left out.
  keyRefusals =
    houses:
    lib.pipe
      (bareNixos [
        self.nixosModules.shaper
        {
          jitsusama.shaper.networks = lib.mapAttrs (_: house: {
            up = 12000;
            house = {
              most = 7000;
            }
            // house;
          }) houses;
        }
      ]).config.assertions
      [
        (lib.filter (check: !check.assertion))
        (map (check: check.message))
        (lib.filter (lib.hasPrefix "jitsusama.shaper.networks"))
      ];
  keyedAt = from: keyFile: { inherit from keyFile; };
  one = "00:00:5e:00:53:01";
  other = "00:00:5e:00:53:02";
  # Keys the shaper's service can't see, under a folder it's given empty
  # or spelled through one, each of which must be refused.
  unseenKeys = [
    "/root/house.key"
    "/home/someone/house.key"
    "/tmp/house.key"
    "/var/tmp/house.key"
    "//root/house.key"
    "/var/lib/./house.key"
    "/var/lib/../../tmp/house.key"
  ];
  takenUnseen = lib.filter (
    path: keyRefusals { ${one} = keyedAt "192.0.2.7" path; } == [ ]
  ) unseenKeys;
  wronglyRefused = keyRefused.plain ++ keyRefused.oneKeyed;
  refusedText = lib.concatStringsSep "; " wronglyRefused;
  keyRefused = {
    plain = keyRefusals { ${one} = keyedAt "192.0.2.7" "/var/lib/house.key"; };
    twoKeys = keyRefusals {
      ${one} = keyedAt "192.0.2.7" "/var/lib/one.key";
      ${other} = keyedAt "192.0.2.7" "/var/lib/other.key";
    };
    oneKeyed = keyRefusals {
      ${one} = keyedAt "192.0.2.7" "/var/lib/one.key";
      ${other}.from = "192.0.2.7";
    };
  };
  # A keyed house naming no address, heard from any by its key alone,
  # opens the port to private IPv4 addresses alone and builds; one naming
  # no key is refused, since nothing would say a report was its.
  anywhereRule =
    "ip saddr { 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 } " + "udp dport ${port} accept";
  anywhere =
    (bareNixos [
      self.nixosModules.shaper
      {
        jitsusama.shaper.networks.${one} = {
          up = 12000;
          house = {
            most = 7000;
            keyFile = "/var/lib/house.key";
          };
        };
      }
    ]).config.networking.firewall;
  anywhereRules = lib.filter (line: line != "") (lib.splitString "\n" anywhere.extraInputRules);
  unaddressed = {
    keyed = keyRefusals { ${one}.keyFile = "/var/lib/house.key"; };
    keyless = keyRefusals { ${one} = { }; };
    said = lib.concatStringsSep "; " unaddressed.keyed;
  };
  unasked = services.shaper.wantedBy == [ ];
  asked = builtins.elem "multi-user.target" named.shaper.wantedBy;
  recorded = builtins.elem "multi-user.target" services.shaper-record.wantedBy;
in
# What follows needs Linux to build or to run, so a Mac leaves it out.
onLinux {
  # The shaper's tests run as it builds: each set, route and queue is read
  # from what the kernel prints, every link to one router counts on its
  # line, a recorded week replays through a controller and is scored, and
  # the one-way controller sets each direction from its own delay.
  shaper-finds-the-rate = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the shaper, its tests included.
  shaper-passes-clippy = passesClippy "shaper-passes-clippy" (pkgs.callPackage ./package.nix { });

  # A switch restarts the shaper and its recorder in place, in its start
  # phase, rather than stopping them before it activates. The shaper's
  # restart keeps its queues (src/restart.rs); the recorder's keeps the
  # record on the shaper's restart, which on 16 November it was blind to
  # from 20:18:46 to 20:18:53 and from 21:51:52 to 21:52:02.
  shaper-stays-through-a-switch =
    holds "shaper-stays-through-a-switch"
      "A switch would stop the shaper or its recorder first, so its queues or its record go."
      (
        services.shaper.restartIfChanged
        && !services.shaper.stopIfChanged
        && services.shaper-record.restartIfChanged
        && !services.shaper-record.stopIfChanged
      );

  # A network's house opens the port the shaper hears its reports on to
  # the house's addresses alone, over IPv4 or IPv6 as each is written: the
  # machine's input rules are the houses' three alone, and no port, range or
  # trusted interface opens it to all. A house's address, alone or in a
  # list, is held to what
  # the shaper's own parse takes, and an IPv4 address written as IPv6,
  # mapped or compatible, which would open the port to no report, is
  # refused.
  shaper-hears-only-the-house = holds "shaper-hears-only-the-house" (
    if !houseRules then
      "The input rules aren't the houses' three alone: ${lib.concatStringsSep "; " rules}"
    else if !opened then
      "UDP port ${port} is open to all, by a port, a range or a trusted interface"
    else
      "A house's address check misreads: ${lib.concatStringsSep ", " misread}"
  ) (opened && misread == [ ]);

  shaper-hears-a-keyed-house-from-any-address =
    holds "shaper-hears-a-keyed-house-from-any-address"
      (
        if anywhereRules != [ anywhereRule ] then
          "A keyed house naming no address doesn't open UDP port ${port} to private IPv4 alone: "
          + lib.concatStringsSep "; " anywhereRules
        else if unaddressed.keyed != [ ] then
          "A keyed house naming no address was refused: ${unaddressed.said}"
        else
          "A house naming neither an address nor a key was taken."
      )
      (
        anywhereRules == [ anywhereRule ]
        && unaddressed.keyed == [ ]
        && lib.any (lib.hasInfix "naming no address") unaddressed.keyless
      );

  # A house's key the shaper's service can't see, under /root, /home,
  # /tmp or /var/tmp or spelled through them, is refused as the machine
  # builds rather than read as missing at run time, holding nothing; and
  # so are two houses naming one address under two keys, while one keyed
  # beside one keyless, which the ear holds to the key, builds.
  shaper-refuses-a-house-key-it-can-t-read = holds "shaper-refuses-a-house-key-it-can-t-read" (
    if takenUnseen != [ ] then
      "A key the shaper can't see was taken: ${lib.concatStringsSep ", " takenUnseen}"
    else if keyRefused.twoKeys == [ ] then
      "Two houses naming one address under two keys were taken."
    else
      "A plain key, or a keyed house beside a keyless one, was refused: ${refusedText}"
  ) (takenUnseen == [ ] && keyRefused.twoKeys != [ ] && wronglyRefused == [ ]);

  # Only a machine that names the controller shapes from boot: every
  # graphical machine imports the module, and one that never asked for
  # the one-way controller was never shaped before it was the only one.
  # Its recorder starts either way, so its week can be replayed first.
  shaper-starts-when-asked = holds "shaper-starts-when-asked" (
    if !unasked then
      "The shaper would start at boot on a machine that never named its controller."
    else if !asked then
      "The shaper wouldn't start at boot on a machine that named its controller."
    else
      "The recorder wouldn't start at boot on a machine that never named the controller."
  ) (unasked && asked && recorded);
}
