# Joins Wi-Fi networks in a virtual machine the way a laptop meets them, with
# wifid, networkd and wpa_supplicant as the laptop role brings them. Two
# stand-in access points live in a network namespace of their own, so their
# traffic crosses the stand-in air rather than taking a shortcut through the
# machine. One network is declared in Nix and one saved as the launcher
# saves it. A joined network gets its address and DNS server and sees an
# address of the card's own for it. wifid finds which network wants a
# sign-in, asking the network's own DNS server even while a VPN's servers
# stand in for it, and notices a guest signing in, a session the network
# said would end ending, and a network letting only the plain check
# through. A resolver for a private domain, set up the way a
# development DNS server is, keeps answering through every join; and a
# routing rule another program made, as a VPN's client does, survives
# joining, reconfiguring and networkd restarting. Nothing that should leave
# the card on its network takes it off: not wifid starting again, nor a
# saved file it can't read, nor another card plugged in and pulled out.
# The shell, as an account of the machine's and no other, is told all of
# it over varlink as it changes, and can have wifid check again, trust a
# network or forget one, though never one Nix declares.
{ self, pkgs }:
let
  inherit (pkgs) lib;

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

  lantern = accessPoint "lantern" ''
    interface=wlan0
    ssid=Lantern
    channel=6
    wpa=2
    wpa_key_mgmt=WPA-PSK
    rsn_pairwise=CCMP
    wpa_passphrase=glowing in the dark
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
    interface=wlan0
    interface=wlan1
    dhcp-range=wlan0,10.42.0.10,10.42.0.50,1h
    dhcp-range=wlan1,10.43.0.10,10.43.0.50,1h
    # Lantern says where to ask it about the session it gives, as RFC 8910
    # lets a network.
    dhcp-option=tag:wlan0,114,"https://api.lantern/captive"
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
  '';

  # Lantern reaches beyond itself, so its check pages answer as Google's
  # do, over HTTP and HTTPS, and its API gives a session a few seconds
  # long. The Inn sends every guest to its front desk until they've signed
  # in there, and answers nothing over HTTPS, as a network letting only the
  # plain check through would.
  checkPage = pkgs.writeText "check-page.py" ''
    import json, ssl, threading, time
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

    LANTERN = "10.42.0.1"
    signed_in = set()
    session = []

    class Page(BaseHTTPRequestHandler):
        def answer(self, code, headers={}, body=b""):
            self.send_response(code)
            for name, value in headers.items():
                self.send_header(name, value)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            network = self.connection.getsockname()[0]
            if self.path == "/sign-in":
                signed_in.add(network)
                return self.answer(200, body=b"Welcome")
            if self.path == "/captive":
                session[:] = session or [time.monotonic()]
                left = 15 - (time.monotonic() - session[0])
                said = {"captive": False, "seconds-remaining": int(left)}
                if left <= 0:
                    said = {"captive": True, "user-portal-url": "https://router.lantern/renew"}
                kind = {"Content-Type": "application/captive+json"}
                return self.answer(200, kind, json.dumps(said).encode())
            if network == LANTERN or network in signed_in:
                return self.answer(204)
            self.answer(302, {"Location": "http://front-desk.inn/sign-in"})

    secure = ThreadingHTTPServer((LANTERN, 443), Page)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain("${certificates}/page.pem", "${certificates}/page.key")
    secure.socket = tls.wrap_socket(secure.socket, server_side=True)
    threading.Thread(target=secure.serve_forever, daemon=True).start()
    ThreadingHTTPServer(("", 80), Page).serve_forever()
  '';
in
pkgs.testers.runNixOSTest {
  name = "wifi";

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

      # Four stand-in radios: two access points, the laptop's card, and a
      # USB card in a drawer.
      boot.kernelModules = [ "mac80211_hwsim" ];
      boot.extraModprobeConfig = "options mac80211_hwsim radios=4";

      # The first two radios move to the access points' namespace and the
      # last to the drawer's before anything joins, leaving wlan2 as the
      # laptop's only card.
      systemd.services.air = {
        after = [ "systemd-modules-load.service" ];
        before = [ "wpa_supplicant.service" ];
        wantedBy = [ "multi-user.target" ];
        path = [
          pkgs.coreutils
          pkgs.iproute2
          pkgs.iw
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
        };
        script = ''
          until [ -e /sys/class/net/wlan3/phy80211/name ]; do sleep 0.1; done
          ip netns add air
          for card in wlan0 wlan1; do
            iw phy "$(cat /sys/class/net/$card/phy80211/name)" set netns name air
          done
          ip netns add drawer
          iw phy "$(cat /sys/class/net/wlan3/phy80211/name)" set netns name drawer
          ip -n air link set lo up
          ip -n air address add 10.42.0.1/24 dev wlan0
          ip -n air address add 10.43.0.1/24 dev wlan1
        '';
      };
      systemd.services.lantern = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${lantern}";
        };
      };
      systemd.services.inn = inAir // {
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${inn}";
        };
      };
      systemd.services.networks = inAir // {
        after = inAir.after ++ [
          "lantern.service"
          "inn.service"
        ];
        before = [ "wifid.service" ];
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe pkgs.dnsmasq} --keep-in-foreground --conf-file=${networks}";
        };
      };

      systemd.services.check-page = inAir // {
        before = [ "wifid.service" ];
        serviceConfig = inAir.serviceConfig // {
          ExecStart = "${lib.getExe pkgs.python3} ${checkPage}";
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

      # Roadside Inn is declared in Nix, whole, the way a machine's own
      # networks are.
      systemd.network.networks."50-wifi-roadside-inn" = {
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

      environment.systemPackages = [
        pkgs.iw
        pkgs.dig
        pkgs.curl
      ];
    };

  testScript = ''
    import json

    # Lantern as the launcher saves it: only what's its own, with its
    # password sealed to this machine.
    lantern = [
        "[Match]",
        "SSID=Lantern",
        "",
        "[X-WiFi]",
        "Security=personal",
        "Secret=/var/lib/wifid/networks/lantern.cred",
        "Priority=10",
    ]
    saved = "/var/lib/wifid/networks/lantern.network"
    given = "/run/systemd/network/50-wifi-lantern.network"

    def holds_address(prefix):
        machine.wait_until_succeeds(
            f"ip -4 -o address show dev wlan2 | grep -q 'inet {prefix}'", timeout=60
        )
        machine.wait_until_succeeds(
            "networkctl status wlan2 | grep -q 'State: routable'", timeout=30
        )

    def on_file(file):
        machine.wait_until_succeeds(
            f"networkctl status wlan2 | grep -q 'Network File: {file}'", timeout=30
        )

    def joined(ssid):
        machine.wait_until_succeeds(
            f"wpa_cli -i wlan2 status | grep -qx 'ssid={ssid}'", timeout=60
        )

    def answers(name, address):
        machine.wait_until_succeeds(
            f"resolvectl query --legend=no {name} | grep -qF {address}", timeout=30
        )

    def rules_stand():
        for family in ("-4", "-6"):
            machine.succeed(
                f"ip {family} rule show | grep -q 'not from all fwmark 0x100cf lookup 65743'"
            )

    def says(line, timeout=30):
        """Waits for wifid, as it runs now, to say line in the journal."""
        machine.wait_until_succeeds(
            "journalctl -o cat _PID=$(systemctl show -P MainPID wifid)"
            f" | grep -qxF '{line}'",
            timeout=timeout,
        )

    def shown():
        """The address the card is showing the network it's on."""
        return machine.succeed("cat /sys/class/net/wlan2/address").strip()

    wifi = "/run/wifid/io.jitsusama.Wifi"

    def calling(method, parameters):
        """How an account of the machine's asks wifid something."""
        return (
            f"runuser -u guest -- varlinkctl call {wifi}"
            f" io.jitsusama.Wifi.{method} '{json.dumps(parameters)}'"
        )

    def ask(method, parameters={}):
        """Asks wifid something as the account would, and reads the answer."""
        return json.loads(machine.succeed(calling(method, parameters)) or "{}")

    def refused(method, parameters, error):
        """Checks wifid refuses, saying why."""
        said = machine.fail(f"{calling(method, parameters)} 2>&1")
        assert f"failed: io.jitsusama.Wifi.{error}" in said, said

    def saved_as(name):
        """The saved network of that name, as wifid tells the shell."""
        return [network for network in ask("Describe")["saved"] if network["name"] == name]

    def journal_now():
        """Where the journal is, to read on from."""
        return machine.succeed(
            "journalctl -n1 --show-cursor -o cat | sed -n 's/^-- cursor: //p'"
        ).strip()

    def stayed_on(since):
        """Checks wpa_supplicant hasn't taken wlan2 off its network since."""
        machine.fail(
            f"journalctl -o cat -u wpa_supplicant --after-cursor='{since}'"
            " | grep 'wlan2: CTRL-EVENT-DISCONNECTED'"
        )

    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("networks.service")
    machine.wait_for_unit("private-dns.service")
    card = machine.succeed("cat /sys/class/net/wlan2/phy80211/macaddress").strip()

    with subtest("wifid gives wpa_supplicant the card, which joins Nix's network"):
        machine.wait_for_unit("wifid.service")
        joined("Roadside Inn")
        holds_address("10.43.0.")
        on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
        answers("front-desk.inn", "10.43.0.1")

    with subtest("wifid finds the Inn wants a sign-in, and where"):
        says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")

    with subtest("and tells the shell, which only an account of the machine's may ask"):
        machine.succeed(f"varlinkctl introspect {wifi} io.jitsusama.Wifi > /tmp/wifi.varlink")
        machine.succeed("varlinkctl validate-idl /tmp/wifi.varlink")
        machine.succeed("varlinkctl list-registry | grep -q io.jitsusama.Wifi")
        described = ask("Describe")
        assert described["cards"] == [
            {
                "name": "wlan2",
                "network": "Roadside Inn",
                "reach": "signIn",
                "signIn": "http://front-desk.inn/sign-in",
                "trouble": None,
            }
        ], described
        inn = saved_as("Roadside Inn")
        assert inn[0]["declared"] and inn[0]["security"] == "open", inn
        machine.fail(
            f"runuser -u stranger -- varlinkctl call {wifi} io.jitsusama.Wifi.Describe '{{}}'"
        )

    with subtest("and what's in the air, looking again when asked"):
        nearby = {network["name"]: network for network in ask("Describe")["nearby"]}
        assert nearby["Roadside Inn"]["security"] == "open", nearby
        assert nearby["Lantern"]["security"] == "personal", nearby
        assert all(1 <= network["strength"] <= 4 for network in nearby.values()), nearby
        # On a network this strong, the card looks again only hourly by
        # itself, so a scan finishing now is the one asked for.
        machine.succeed(
            "busctl monitor --match \"type='signal',member='ScanDone'\" > /tmp/scans 2>&1 &"
        )
        machine.wait_until_succeeds("grep -q Monitoring /tmp/scans", timeout=10)
        ask("Scan")
        machine.wait_until_succeeds("grep -q ScanDone /tmp/scans", timeout=30)
        machine.succeed("pkill -f 'busctl monitor'")

    with subtest("the network sees an address the card chose for it"):
        inn_address = shown()
        assert inn_address != card, inn_address
        assert int(inn_address[:2], 16) & 0b11 == 0b10, inn_address

    with subtest("a network saved from the launcher is laid over the defaults and chosen first"):
        lines = " ".join(f"'{line}'" for line in lantern)
        machine.succeed(f"printf '%s\\n' {lines} > {saved}")
        machine.succeed(
            "printf 'glowing in the dark' | systemd-creds encrypt --name=lantern"
            " - /var/lib/wifid/networks/lantern.cred"
        )
        machine.succeed("systemctl restart wifid")
        # The card stays where it is until it next chooses, as the
        # launcher's join will have it do.
        machine.wait_until_succeeds(f"test -e {given}", timeout=30)
        joined("Roadside Inn")
        machine.succeed("wpa_cli -i wlan2 reassociate")
        joined("Lantern")
        holds_address("10.42.0.")
        on_file(given)
        written = machine.succeed(f"cat {given}")
        for line in ("Type=wlan", "SSID=Lantern", "IPv6PrivacyExtensions=yes", "[X-WiFi]"):
            assert line in written, written
        answers("router.lantern", "10.42.0.1")
        # networkd knows the network by name, which a saved network's file
        # is matched on.
        machine.succeed("networkctl status wlan2 | grep -q 'Wi-Fi access point: Lantern '")
        answers("probe.corp.internal", "10.99.0.7")
        lantern_address = shown()
        assert lantern_address not in (inn_address, card), lantern_address

    with subtest("and finds Lantern reaches beyond itself"):
        says("Lantern on wlan2 reaches beyond itself")

    with subtest("and asks Lantern again as the session it gave ends"):
        says("Lantern on wlan2 wants a sign-in at https://router.lantern/renew", timeout=45)

    with subtest("a stranger's network gets no LLMNR or mDNS"):
        status = machine.succeed("resolvectl status wlan2")
        assert "-LLMNR" in status and "-mDNS" in status, status

    with subtest("trusting it from the shell turns on mDNS, and the card stays on it"):
        since = journal_now()
        ask("Trust", {"name": "Lantern", "trusted": True})
        machine.wait_until_succeeds("resolvectl status wlan2 | grep -qF +mDNS", timeout=30)
        machine.succeed(f"grep -qx MulticastDNS=yes {saved}")
        assert saved_as("Lantern")[0]["trusted"], saved_as("Lantern")
        ask("Trust", {"name": "Lantern", "trusted": False})
        machine.wait_until_succeeds("resolvectl status wlan2 | grep -qF -- -mDNS", timeout=30)
        assert not saved_as("Lantern")[0]["trusted"], saved_as("Lantern")
        holds_address("10.42.0.")
        stayed_on(since)
        refused("Trust", {"name": "Roadside Inn", "trusted": True}, "Declared")
        refused("Trust", {"name": "Nowhere", "trusted": True}, "NotSaved")

    # WARP's own rule, at its own priority, for both families.
    for family in ("-4", "-6"):
        machine.succeed(f"ip {family} rule add not fwmark 0x100cf lookup 65743 priority 32765")

    with subtest("another program's routing rule survives a reconfigure"):
        machine.succeed("networkctl reconfigure wlan2")
        holds_address("10.42.0.")
        rules_stand()

    with subtest("it survives moving to another network when one goes away"):
        machine.systemctl("stop lantern")
        joined("Roadside Inn")
        holds_address("10.43.0.")
        on_file("/etc/systemd/network/50-wifi-roadside-inn.network")
        rules_stand()
        answers("front-desk.inn", "10.43.0.1")
        answers("probe.corp.internal", "10.99.0.7")

    with subtest("a network sees the same address every time"):
        assert shown() == inn_address, shown()

    with subtest("wifid starting again leaves the card on its network"):
        garbled = "/var/lib/wifid/networks/garbled.network"
        machine.succeed(f"printf '[\\377]' > {garbled}")
        since = journal_now()
        machine.succeed("systemctl restart wifid")
        says(f"{garbled}: stream did not contain valid UTF-8")
        says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
        stayed_on(since)
        machine.succeed(f"rm {garbled}")

    with subtest("another card coming and going leaves the first on its network"):
        supplicant = machine.succeed("systemctl show -P MainPID wpa_supplicant")
        since = journal_now()
        radio = machine.succeed(
            "ip netns exec drawer iw dev wlan3 info | sed -n 's/^.*wiphy /phy/p'"
        ).strip()
        machine.succeed(f"ip netns exec drawer iw phy {radio} set netns 1")
        machine.wait_until_succeeds("wpa_cli -i wlan3 ping | grep -qx PONG", timeout=30)
        machine.succeed(f"iw phy {radio} set netns name drawer")
        machine.wait_until_succeeds("! test -e /sys/class/net/wlan3", timeout=30)
        assert machine.succeed("systemctl show -P MainPID wpa_supplicant") == supplicant
        stayed_on(since)
        joined("Roadside Inn")

    with subtest("and networkd restarting"):
        machine.succeed("systemctl restart systemd-networkd")
        holds_address("10.43.0.")
        rules_stand()
        answers("probe.corp.internal", "10.99.0.7")

    with subtest("wifid adds the card again when wpa_supplicant restarts"):
        machine.succeed("systemctl restart wpa_supplicant")
        machine.wait_until_succeeds("wpa_cli -i wlan2 ping | grep -qx PONG", timeout=30)
        joined("Roadside Inn")
        holds_address("10.43.0.")

    with subtest("the check asks the network's own DNS server while a VPN's stand in"):
        # WARP points the link at its own servers while it's connected.
        machine.succeed("resolvectl dns wlan2 192.0.2.53")
        machine.succeed("resolvectl dns wlan2 | grep -q '192.0.2.53$'")
        machine.succeed("systemctl restart wifid")
        says("Roadside Inn on wlan2 wants a sign-in at http://front-desk.inn/sign-in")
        machine.succeed("resolvectl revert wlan2")

    with subtest("forgetting Lantern takes it from everywhere, and the card stays on the Inn"):
        since = journal_now()
        ask("Forget", {"name": "Lantern"})
        for gone in (saved, given, "/var/lib/wifid/networks/lantern.cred"):
            machine.succeed(f"! test -e {gone}")
        held = machine.succeed("wpa_cli -i wlan2 list_networks")
        assert "Roadside Inn" in held and "Lantern" not in held, held
        assert saved_as("Lantern") == [], saved_as("Lantern")
        stayed_on(since)
        refused("Forget", {"name": "Lantern"}, "NotSaved")
        refused("Forget", {"name": "Roadside Inn"}, "Declared")
        joined("Roadside Inn")

    with subtest("networkd's file for a network no longer saved is taken away"):
        gone = "/run/systemd/network/50-wifi-gone.network"
        machine.succeed(
            f"printf '# Written by wifid from /var/lib/wifid/networks/gone.network\\n' > {gone}"
        )
        machine.succeed("systemctl restart wifid")
        machine.wait_for_unit("wifid.service")
        machine.wait_until_succeeds(f"! test -e {gone}", timeout=30)

    with subtest("signing in at the Inn is noticed when asked, and the shell told"):
        machine.succeed(
            f"runuser -u guest -- varlinkctl -E --json=short call {wifi}"
            " io.jitsusama.Wifi.Describe '{}' > /tmp/described 2>&1 &"
        )
        machine.wait_until_succeeds("grep -q signIn /tmp/described", timeout=10)
        machine.succeed("curl -sf --interface wlan2 http://front-desk.inn/sign-in")
        ask("Check")
        # Well before the half-minute the Inn would otherwise wait.
        says("Roadside Inn on wlan2 reaches beyond itself", timeout=15)
        machine.wait_until_succeeds(
            "tail -n1 /tmp/described | grep -qF '\"reach\":\"online\"'", timeout=10
        )

    with subtest("a network letting only the plain check through reaches no further"):
        machine.succeed("systemctl restart wifid")
        says("Roadside Inn on wlan2 reaches no further")

    def hears(name):
        """Has the cards look, and says whether they hear the network."""
        ask("Scan")
        return any(network["name"] == name for network in ask("Describe")["nearby"])

    def joining(parameters):
        """Joins as the launcher does, giving each step wifid said, or why not."""
        call = calling("Join", parameters).replace("call", "call --more --json=short", 1)
        status, said = machine.execute(f"{call} 2>&1")
        # An error's parameters are printed too, and have no stage.
        replies = [json.loads(line) for line in said.splitlines() if line.startswith("{")]
        stages = [reply["stage"] for reply in replies if "stage" in reply]
        return status, stages, said

    sealed = "/var/lib/wifid/networks/lantern.cred"

    with subtest("joining Lantern from the launcher with the wrong password saves nothing"):
        machine.systemctl("start lantern")
        retry(lambda _: hears("Lantern"), timeout_seconds=60)
        refused("Join", {"name": "Lantern"}, "NeedsPassword")
        refused("Join", {"name": "Nowhere"}, "NotNearby")
        status, stages, said = joining({"name": "Lantern", "password": "not the password"})
        assert status != 0 and "failed: io.jitsusama.Wifi.WrongPassword" in said, said
        assert stages[:1] == ["joining"] and "online" not in stages, said
        assert saved_as("Lantern") == [], saved_as("Lantern")
        machine.succeed(f"! test -e {sealed}")
        joined("Roadside Inn")

    with subtest("and with the right one, it's saved sealed and said joined step by step"):
        status, stages, said = joining({"name": "Lantern", "password": "glowing in the dark"})
        assert status == 0, said
        # Lantern's session ran out long ago, so it wants a sign-in again.
        assert stages[0] == "joining" and stages[-1] == "signIn", said
        joined("Lantern")
        holds_address("10.42.0.")
        lantern = saved_as("Lantern")
        assert lantern and not lantern[0]["declared"], lantern
        held = machine.succeed(f"cat {sealed}")
        assert "glowing" not in held, held
        held = machine.succeed("wpa_cli -i wlan2 list_networks")
        assert "[DISABLED]" not in held, held

    with subtest("joining the Inn again moves the card, and says how far the Inn reaches"):
        status, stages, said = joining({"name": "Roadside Inn"})
        assert status == 0 and stages[-1] == "limited", said
        joined("Roadside Inn")

    def on_network(name):
        """Whether wifid tells the shell a card is on the network."""
        return any(card["network"] == name for card in ask("Describe")["cards"])

    with subtest("leaving the Inn from the shell keeps the card off it until it's joined"):
        ask("Leave", {"name": "Roadside Inn"})
        machine.wait_until_succeeds(
            "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
        )
        retry(lambda _: not on_network("Roadside Inn"), timeout_seconds=30)
        refused("Leave", {"name": "Roadside Inn"}, "NotJoined")
        machine.sleep(10)
        machine.succeed("wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED")
        status, stages, said = joining({"name": "Roadside Inn"})
        assert status == 0, said
        joined("Roadside Inn")

    with subtest("the shell is told whether a cable reaches beyond the machine"):
        # The test's own network and QEMU's are both cables here.
        links = machine.succeed("networkctl list --no-legend")
        cables = [line.split()[1] for line in links.splitlines() if line.split()[2] == "ether"]
        assert "routable" in links and cables, links
        retry(lambda _: ask("Describe")["wired"] is True, timeout_seconds=30)
        for cable in cables:
            machine.succeed(f"ip link set {cable} down")
        retry(lambda _: ask("Describe")["wired"] is False, timeout_seconds=30)
        for cable in cables:
            machine.succeed(f"ip link set {cable} up")
        retry(lambda _: ask("Describe")["wired"] is True, timeout_seconds=60)

    with subtest("turning Wi-Fi off from the shell switches the radios off, and on again"):
        assert ask("Describe")["radio"], ask("Describe")
        # Off its network first, so nothing but wifid has networkd set the
        # card's link up again once the radios are back.
        ask("Leave", {"name": "Roadside Inn"})
        machine.wait_until_succeeds(
            "wpa_cli -i wlan2 status | grep -qx wpa_state=DISCONNECTED", timeout=30
        )
        ask("Radio", {"on": False})
        machine.wait_until_succeeds("rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=10)
        retry(lambda _: ask("Describe")["radio"] is False, timeout_seconds=30)
        ask("Radio", {"on": True})
        machine.wait_until_succeeds("! rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=10)
        retry(lambda _: ask("Describe")["radio"] is True, timeout_seconds=30)
        retry(lambda _: ",UP" in machine.succeed("ip link show wlan2"), timeout_seconds=30)
        status, stages, said = joining({"name": "Roadside Inn"})
        assert status == 0, said
        joined("Roadside Inn")
  '';
}
