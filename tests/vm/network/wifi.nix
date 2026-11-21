# Joins Wi-Fi networks in a virtual machine the way a laptop meets them, with
# wifid, networkd and wpa_supplicant as the laptop role brings them. Two
# stand-in access points live in a network namespace of their own, so their
# traffic crosses the stand-in air rather than taking a shortcut through the
# machine. One network is declared in Nix, eight more out of range with
# passwords systemd sealed, and one saved as the launcher saved it before
# wifid had a vault, which wifid moves into it; after that, no card wifid
# is given reaches systemd for a password. A joined network gets its address and DNS server and sees an
# address of the card's own for it. wifid finds which network wants a
# sign-in, asking the network's own DNS server even while a VPN's servers
# stand in for it, and notices a guest signing in, a session the network
# said would end ending, and a network letting only the plain check
# through. A resolver for a private domain, set up the way a
# development DNS server is, keeps answering through every join; and a
# routing rule another program made, as a VPN's client does, survives
# joining, reconfiguring and networkd restarting. Nothing that should leave
# the card on its network takes it off: not wifid starting again, nor a
# saved file it can't read, nor another card plugged in and pulled out,
# which looks for networks from made-up addresses with its link up. The
# boot waits on wifid to say there's no more to wait for. The card stays
# awake while a charger is plugged in, and dozes once both are pulled.
# The shell, as an account of the machine's and no other, is told all of
# it over varlink as it changes, and can have wifid check again, trust a
# network or forget one, though never one Nix declares; on the launcher's
# rows socket it can move the card to another access point of its network,
# and the card still knows its neighbours there, forgetting them only on a
# network of another name; a move to one gone takes it back to the one it
# left within wifid's 2 s for a way back, past any wait that one bids, or
# leaves it free to join any network when it can't get back either. The
# tests are in wifi/, an area to a file, each run in order through pytest
# on a machine of its own, so the areas run side by side; wifid as they
# reach it is wifi/wifi_session.py.
{
  self,
  pkgs,
  # Which area's tests run: joins, holds-on, joins-when-asked or moves.
  area,
}:
let
  inherit (pkgs) lib;

  lighthouses = map (n: "lighthouse-${toString n}") (lib.range 1 8);

  # The access points' side, run inside their namespace.
  inAir = {
    after = [ "air.service" ];
    requires = [ "air.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig.NetworkNamespacePath = "/run/netns/air";
  };

  # Each names its country, as nearly every real access point does. The
  # test runs nixpkgs's kernel, which records the name of a network joined
  # only when it does; core.nix's kernel has a patch for the others.
  accessPoint =
    name: settings:
    pkgs.writeText "${name}.conf" ''
      driver=nl80211
      hw_mode=g
      country_code=CA
      ieee80211d=1
      ${settings}
    '';

  # WPA2 and WPA3 both, as a home network's often is; the card takes WPA3,
  # by hash-to-element.
  lanternKey = ''
    ssid=Lantern
    wpa=2
    wpa_key_mgmt=WPA-PSK SAE
    rsn_pairwise=CCMP
    ieee80211w=1
    sae_pwe=2
    wpa_passphrase=glowing in the dark
  '';

  # Both of Lantern's access points join one bridge, where its router
  # answers, as a home's do, so a card moving between them keeps the same
  # router at the same hardware address.
  lantern = accessPoint "lantern" ''
    interface=wlan0
    bridge=lantern
    channel=6
    ${lanternKey}
  '';

  # Lantern's second access point, heard only when a test starts it. It
  # can ask a card to move, as a mesh router's points do, through its
  # control socket.
  lanternTwo = accessPoint "lantern-two" ''
    interface=wlan5
    bridge=lantern
    channel=9
    bss_transition=1
    ctrl_interface=/run/hostapd-lantern-two
    ${lanternKey}
  '';

  inn = accessPoint "inn" ''
    interface=wlan1
    ssid=Roadside Inn
    channel=11
  '';

  # Each network hands out addresses and answers for its own names.
  networks = pkgs.writeText "networks.conf" ''
    bind-dynamic
    dhcp-leasefile=/run/networks.leases
    interface=lantern
    interface=wlan1
    interface=wlan4
    dhcp-range=lantern,10.42.0.10,10.42.0.50,1h
    dhcp-range=wlan1,10.43.0.10,10.43.0.50,1h
    dhcp-range=wlan4,10.44.0.10,10.44.0.50,1h
    # Lantern says where to ask it about the session it gives, as RFC 8910
    # lets a network.
    dhcp-option=tag:lantern,114,"https://api.lantern/captive"
    address=/router.lantern/10.42.0.1
    address=/api.lantern/10.42.0.1
    address=/front-desk.inn/10.43.0.1
    no-resolv
    # The check page is on each network's own router, so a check finds the
    # one it meant only by asking that network's server.
    addn-hosts=${checkPageAddresses}
    localise-queries
  '';

  checkPageAddresses = pkgs.writeText "check-page" ''
    10.42.0.1 connectivitycheck.gstatic.com secure-check.test
    10.43.0.1 connectivitycheck.gstatic.com secure-check.test
  '';

  # A certificate authority the machine trusts, standing in for the real
  # ones, and a certificate from it for Lantern's HTTPS pages.
  certificates = pkgs.runCommand "stand-in-certificates" { nativeBuildInputs = [ pkgs.openssl ]; } ''
    mkdir $out
    cd $out
    key="-newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes"
    openssl req -x509 $key -days 3650 -subj /CN=Stand-in -keyout authority.key -out authority.pem \
      -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign
    openssl req $key -subj /CN=api.lantern -keyout page.key -out page.csr
    printf '%s\n' subjectAltName=DNS:api.lantern,DNS:secure-check.test \
      extendedKeyUsage=serverAuth > page.ext
    openssl x509 -req -in page.csr -CA authority.pem -CAkey authority.key -CAcreateserial \
      -days 3650 -extfile page.ext -out page.pem
    openssl req $key -subj /CN=radius.office.test -keyout radius.key -out radius.csr
    printf '%s\n' subjectAltName=DNS:radius.office.test extendedKeyUsage=serverAuth > radius.ext
    openssl x509 -req -in radius.csr -CA authority.pem -CAkey authority.key -CAcreateserial \
      -days 3650 -extfile radius.ext -out radius.pem
  '';

  # The Office asks for an account, as a workplace's network does, and its
  # server proves itself with a certificate naming radius.office.test.
  office = accessPoint "office" ''
    interface=wlan4
    ssid=Office
    channel=1
    wpa=2
    wpa_key_mgmt=WPA-EAP
    rsn_pairwise=CCMP
    ieee8021x=1
    eap_server=1
    eap_user_file=${pkgs.writeText "office-accounts" ''
      * PEAP,TTLS
      "joel" MSCHAPV2,TTLS-PAP,TTLS-MSCHAPV2 "hunter22" [2]
    ''}
    ca_cert=${certificates}/authority.pem
    server_cert=${certificates}/radius.pem
    private_key=${certificates}/radius.key
  '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "wifi-${area}";
  extraPythonPackages = python: [
    (python.toPythonModule (
      pkgs.runCommandLocal "wifi-session" { } ''
        mkdir -p $out/${python.python.sitePackages}
        cp ${./wifi/wifi_session.py} $out/${python.python.sitePackages}/wifi_session.py
      ''
    ))
  ];

  nodes.machine =
    { pkgs, ... }:
    {
      imports = [
        self.nixosModules.home-manager
        self.nixosModules.wifid
      ];

      # An account home-manager keeps, which may ask wifid anything, and one
      # it doesn't, which may not.
      users.users.guest.isNormalUser = true;
      home-manager.users.guest.home.stateVersion = "26.05";
      users.users.stranger.isNormalUser = true;

      security.pki.certificateFiles = [ "${certificates}/authority.pem" ];
      # Google's HTTPS page is out of reach here, so the check asks the
      # networks' own.
      environment.etc."wifid/wifid.conf".source = lib.mkForce (
        pkgs.writeText "wifid.conf" ''
          [Check]
          SecurePage=https://secure-check.test/generate_204
        ''
      );

      # Six stand-in radios: two access points, the laptop's card, a USB
      # card in a drawer, the Office's access point and Lantern's second.
      # And the kernel's stand-in battery, with its chargers, which start on.
      boot.kernelModules = [
        "mac80211_hwsim"
        "test_power"
      ];
      boot.extraModprobeConfig = "options mac80211_hwsim radios=6";

      # The first two radios and the last two move to the access points'
      # namespace and the fourth to the drawer's before anything joins,
      # leaving wlan2 as the laptop's only card.
      systemd.services.air = {
        after = [ "systemd-modules-load.service" ];
        before = [ "wpa_supplicant.service" ];
        wantedBy = [ "multi-user.target" ];
        path = [
          pkgs.coreutils
          pkgs.iproute2
          pkgs.iw
          pkgs.nftables
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
        };
        script = ''
          until [ -e /sys/class/net/wlan5/phy80211/name ]; do sleep 0.1; done
          ip netns add air
          for card in wlan0 wlan1 wlan4 wlan5; do
            iw phy "$(cat /sys/class/net/$card/phy80211/name)" set netns name air
          done
          ip netns add drawer
          iw phy "$(cat /sys/class/net/wlan3/phy80211/name)" set netns name drawer
          ip -n air link set lo up
          # A bridge left to itself takes the lowest address of its access
          # points, so the router's would change as they come and go.
          ip -n air link add lantern address 02:00:00:00:42:01 type bridge
          ip -n air link set lantern up
          ip -n air address add 10.42.0.1/24 dev lantern
          ip -n air address add 10.43.0.1/24 dev wlan1
          ip -n air address add 10.44.0.1/24 dev wlan4
          # The gateway counts the echoes reaching it marked for video, and
          # its answers leaving marked alike, so a test can see a card's
          # marked echo time the video queue both ways.
          ip netns exec air nft add table ip marked
          ip netns exec air nft add chain ip marked input '{ type filter hook input priority 0; }'
          ip netns exec air nft add rule ip marked input ip dscp cs5 icmp type echo-request counter
          ip netns exec air nft add chain ip marked output '{ type filter hook output priority 0; }'
          ip netns exec air nft add rule ip marked output ip dscp cs5 icmp type echo-reply counter
        '';
      };
      systemd.services.lantern = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${lantern}";
        };
      };
      # Weaker than the first, so a card hearing both picks the first.
      systemd.services.lantern-two = inAir // {
        wantedBy = [ ];
        serviceConfig = inAir.serviceConfig // {
          ExecStartPre = "${lib.getExe' pkgs.iw "iw"} dev wlan5 set txpower fixed 500";
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${lanternTwo}";
        };
      };
      systemd.services.inn = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${inn}";
        };
      };
      systemd.services.office = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${office}";
        };
      };
      systemd.services.networks = inAir // {
        after = inAir.after ++ [
          "lantern.service"
          "inn.service"
          "office.service"
        ];
        before = [ "wifid.service" ];
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe pkgs.dnsmasq} --keep-in-foreground --conf-file=${networks}";
        };
      };

      # The networks' check and sign-in pages, given the certificates
      # Lantern's HTTPS pages prove themselves with.
      systemd.services.check-page = inAir // {
        before = [ "wifid.service" ];
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe pkgs.python3} ${./wifi-check-page.py} ${certificates}";
        };
      };

      # A resolver for a private domain, set up the way dev's is: a global
      # server in resolved.conf.d for its domain alone, on a local port.
      environment.etc."systemd/resolved.conf.d/private.conf".text = ''
        [Resolve]
        DNS=127.0.0.1:5553
        Domains=~corp.internal
      '';
      systemd.services.private-dns = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig.ExecStart = lib.concatStringsSep " " [
          (lib.getExe pkgs.dnsmasq)
          "--keep-in-foreground --no-resolv --no-hosts --bind-interfaces"
          "--listen-address=127.0.0.1 --port=5553"
          "--address=/corp.internal/10.99.0.7"
        ];
      };

      # The Inn's card is kept off the Office's and Lantern's channels, 1 to
      # 6, the way a machine keeps its home network off a crowded one; the
      # Inn's own, 11, stays.
      jitsusama.wifid.avoid."Roadside Inn" = [
        {
          from = 2412;
          to = 2437;
        }
      ];

      # Roadside Inn is declared in Nix, whole, the way a machine's own
      # networks are. Eight lighthouses more, out of range, each with a
      # password systemd sealed to the machine as it boots, the way a
      # machine's own secrets are kept.
      systemd.network.networks = {
        "50-wifi-roadside-inn" = {
          matchConfig.SSID = ''"Roadside Inn"'';
          networkConfig = {
            DHCP = "yes";
            LLMNR = "no";
            MulticastDNS = "no";
          };
          extraConfig = ''
            [X-WiFi]
            Security=open
          '';
        };
      }
      // lib.listToAttrs (
        map (
          lighthouse:
          lib.nameValuePair "50-wifi-${lighthouse}" {
            matchConfig.SSID = ''"${lighthouse}"'';
            extraConfig = ''
              [X-WiFi]
              Security=personal
              Secret=/var/lib/lighthouses/${lighthouse}.cred
            '';
          }
        ) lighthouses
      );

      # The network is waited for only by what wants it online, as a VPN's
      # client does at boot.
      systemd.services.needs-network = {
        wantedBy = [ "multi-user.target" ];
        wants = [ "network-online.target" ];
        after = [ "network-online.target" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = "${pkgs.coreutils}/bin/true";
        };
      };

      # The lighthouses' passwords, which wifid opens through systemd once,
      # however many cards it has.
      systemd.services.lighthouses = {
        wantedBy = [ "wifid.service" ];
        before = [ "wifid.service" ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          StateDirectory = "lighthouses";
        };
        script = lib.concatMapStrings (lighthouse: ''
          printf 'keeper of the light' | ${pkgs.systemd}/bin/systemd-creds encrypt \
            --name=${lighthouse} - /var/lib/lighthouses/${lighthouse}.cred
        '') lighthouses;
      };

      environment.systemPackages = [
        pkgs.iw
        pkgs.dig
        pkgs.curl
        # For reading the gateway's count of echoes marked for video.
        pkgs.nftables
        # For asking acts on the launcher's rows socket as the launcher does.
        pkgs.socat
        # For an access point's ask to move a card, by hostapd_cli.
        pkgs.hostapd
      ];
    };

  tests = ./wifi/wifi-${area}-vm-test.py;
}
