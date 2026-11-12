// The cases of warp for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Warp.js" as Warp
.import "../WarpDrawer.js" as WarpDrawer
.import "../WarpSeen.js" as WarpSeen
.import "../WarpSwitch.js" as WarpSwitch
.import "../WarpWaiting.js" as WarpWaiting

function run(failures, fromRoot) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // WARP, from the record warp prints (modules/warp/warp.json), the
    // same one its own tests read warp-cli's answers into, in the
    // words the launcher's vocabulary gives it (launcher/words.json).
    const reading = file => {
        const request = new XMLHttpRequest();
        request.open("GET", fromRoot.url(file), false);
        request.send();
        return JSON.parse(request.responseText);
    };
    const warp = reading("warp.json");
    const vocabulary = reading("words.json").things;
    const warpSaid = vocabulary.find(thing => thing.id === "warp").said;
    const warpTold = record => Warp.told(record, {}, warpSaid);
    const figure = (id, side, note, copy) => ({ id, side, note, copy, warns: false });
    // An entry of a list, a host or an address, named by itself.
    const entry = (id, name, side) => ({ id, name, side, note: "", copy: name, warns: false });
    const linked = warpTold(warp);
    machine("WARP connected, as its row says it", linked && [linked.headline, linked.warns],
        ["connected \u00b7 127 ms", false]);
    machine("WARP's parts", linked?.parts, [
        figure("state", "connected", "", "connected"),
        figure("health", "healthy", "", "healthy"),
        Object.assign(figure("settings", "warp+doh", "always on \u00b7 split tunnel",
            "warp+doh"), { parts: [
            figure("settings:mode", "warp+doh",
                "the tunnel with DNS over HTTPS \u00b7 set by local policy", "warp+doh"),
            figure("settings:always-on", "on", "the default", "on"),
            Object.assign(figure("settings:split-tunnel", "include",
                "2 hosts \u00b7 2 networks \u00b7 set by network policy", "include"), { parts: [
                entry("settings:split-tunnel:host", "intranet.example.com", "example"),
                entry("settings:split-tunnel:host", "*.corp.example.com", "example"),
                entry("settings:split-tunnel:network", "10.0.0.0/8", "lan"),
                entry("settings:split-tunnel:network", "2001:db8:1::/48", "lan")
            ] }),
            figure("settings:switch-locked", "off", "set by an override", "off"),
            figure("settings:auto-connect", "2 min", "set by local policy", "2"),
            figure("settings:organisation", "example", "set by local policy", "example"),
            figure("settings:tunnel-protocol", "masque", "set by network policy", "masque"),
            figure("settings:post-quantum", "enabled with downgrades",
                "set by network policy", "enabled_with_downgrades"),
            Object.assign(figure("settings:doh", "0000.doh.example.com", "4 addresses",
                "0000.doh.example.com"), { parts: [
                entry("settings:doh:address", "192.0.2.53", ""),
                entry("settings:doh:address", "192.0.2.54", ""),
                entry("settings:doh:address", "2001:db8::53", ""),
                entry("settings:doh:address", "2001:db8::54", "")
            ] }),
            Object.assign(figure("settings:local-domains", "3 domains", "", "3"), { parts: [
                entry("settings:local-domains:domain", "home.arpa", ""),
                entry("settings:local-domains:domain", "local", ""),
                entry("settings:local-domains:domain", "example.com", "192.0.2.1")
            ] }),
            figure("settings:sign-in-pages", "on", "opens its firewall for 5 min", "on"),
            figure("settings:path-mtu", "on", "", "on"),
            figure("settings:mode-switch", "on", "", "on"),
            figure("settings:may-leave", "off", "", "off"),
            figure("settings:local-network", "/24", "", "24"),
            figure("settings:override", "off", "", "off")
        ] }),
        figure("latency", "127 ms", "", "127"),
        figure("loss", "1.24%", "", "0.012355098"),
        figure("protocol", "MASQUE", "HTTPS via UDP", "MASQUE (HTTPS via UDP)"),
        figure("colo", "YYZ", "metal 0a1b2c", "YYZ"),
        figure("endpoint", "198.51.100.2", "no IPv6", "198.51.100.2"),
        figure("handshake", "2 h ago", "", "8751"),
        figure("traffic", "10.2 GB in \u00b7 9.26 GB out", "",
            "10.2 GB in \u00b7 9.26 GB out"),
        Object.assign(figure("tls", "TLSv1.3", "post-quantum", "TLSv1.3"), { parts: [
            figure("tls:version", "TLSv1.3", "", "TLSv1.3"),
            figure("tls:curve", "P256Kyber768Draft00", "", "P256Kyber768Draft00"),
            figure("tls:cipher", "TLS_AES_256_GCM_SHA384", "", "TLS_AES_256_GCM_SHA384"),
            figure("tls:post-quantum", "on", "", "on"),
            figure("tls:server-name", "masque.example.com", "", "masque.example.com")
        ] }),
        Object.assign(figure("dns", "134 ms", "8 of 12 answered", "134.08333333333334"),
            { parts: [
                figure("dns:proxy", "on", "", "on"),
                figure("dns:average", "134 ms", "", "134.08333333333334"),
                figure("dns:asked", "12", "", "12"),
                figure("dns:answered", "8", "", "8"),
                figure("dns:no-record", "4", "", "4"),
                figure("dns:timed-out", "0", "", "0"),
                figure("dns:failed", "0", "", "0")
            ] }),
        // The work network names what carries it once, last (VKCF3Z,
        // 2026-11-11).
        figure("through", "WARP", "", "WARP")
    ]);
    // Every part is named by the vocabulary, inside the part that holds
    // it, so none reads as its id.
    const nameless = [];
    const partsNamed = (parts, within) => {
        for (const part of parts ?? []) {
            // An entry of a list is named by itself, as a host is.
            const word = vocabulary.find(thing => thing.id === `warp:${part.id}`);
            if (part.name === undefined && (!word?.name || word.in !== within)) {
                nameless.push(part.id);
            }
            partsNamed(part.parts, `warp:${part.id}`);
        }
    };
    partsNamed(linked?.parts, "warp");
    machine("every part of WARP named where it is", nameless, []);
    // Named as Network names the same figures elsewhere, Traffic's
    // Round Trip among them (VKCF3Z and YI77D3, 2026-11-08).
    machine("WARP's figures named as Network names them",
        ["latency", "colo", "traffic"].map(id =>
            vocabulary.find(thing => thing.id === `warp:${id}`)?.name),
        ["Round Trip", "Data Centre", "Traffic"]);
    // A run of items joins with dots, never commas (docs/design.md,
    // Typesetting).
    const commas = [];
    const noCommas = parts => {
        for (const part of parts ?? []) {
            if (`${part.side}${part.note}`.includes(",")) {
                commas.push(part.id);
            }
            noCommas(part.parts);
        }
    };
    noCommas(linked?.parts);
    machine("no part of WARP joins its words with a comma", commas, []);
    const slight = warpTold(Object.assign({}, warp, { tunnel: Object.assign({}, warp.tunnel,
        { loss_fraction: 0.0053708, received_bytes: null }) }));
    machine("a slight loss keeps three digits, and one way's traffic says only it",
        slight?.parts.filter(part => ["loss", "traffic"].includes(part.id))
            .map(part => part.side),
        ["0.537%", "9.26 GB out"]);
    const degraded = warpTold(Object.assign({}, warp, { status: { state: "Connected",
        reason: "NetworkDegraded", reason_figures: { loss_percent: 5.3708224, rtt_ms: 170 } } }));
    machine("WARP degraded, as its row says it", degraded && [degraded.headline, degraded.warns],
        ["degraded \u00b7 127 ms", true]);
    // Its state alone at the side, which never gives way; its figures in
    // its note, whose they are first, since the note gives way from its
    // end and Round Trip and Loss below are the tunnel's own (VKCF3Z).
    machine("WARP's health degraded, saying whose measure first",
        degraded?.parts.find(part => part.id === "health"),
        { id: "health", side: "degraded",
            note: "WARP's measure \u00b7 5.37% loss \u00b7 170 ms round trip",
            copy: "degraded", warns: true });
    const down = warpTold({ status: { state: "Disconnected", reason: "Manual",
        reason_figures: null }, tunnel: null, settings: null, dns: null });
    machine("WARP disconnected, its tunnel unknown", down && [down.headline, down.parts], [
        "disconnected",
        [figure("state", "disconnected", "manual", "disconnected"),
            figure("through", "WARP", "", "WARP")]
    ]);
    const connecting = warpTold({ status: { state: "Connecting",
        reason: "PerformingHappyEyeballs", reason_figures: null }, tunnel: null });
    machine("WARP connecting, saying why", connecting?.parts[0].note,
        "performing happy eyeballs");
    const thin = warpTold(Object.assign({}, warp, { tunnel: { on: true, latency_ms: null,
        loss_fraction: null, protocol: "WireGuard", edge: null, sent_bytes: null,
        received_bytes: null }, settings: null, dns: null }));
    machine("a figure warp didn't give is left out",
        thin?.parts.map(part => part.id), ["state", "health", "protocol", "through"]);
    const off = warpTold(Object.assign({}, warp, { tunnel: { on: false, v4_endpoint: null,
        v6_endpoint: "2001:db8::2" } }));
    machine("a tunnel that's off, its one endpoint IPv6",
        off?.parts.filter(part => ["tunnel", "endpoint"].includes(part.id)), [
        figure("tunnel", "off", "", "off"),
        figure("endpoint", "2001:db8::2", "no IPv4", "2001:db8::2")
    ]);
    const policed = warpTold(Object.assign({}, warp, { settings: { mode: "tunnel_only",
        always_on: false, split_tunnel: { mode: "exclude", hosts: 1, ips: 0 },
        set_by: { mode: "someone_else" } }, override: null,
        dns: { proxy_enabled: false, average_ms: null,
        total: 0, success: 0, no_records_found: 0, timed_out: 0, other_error: 0 } }));
    machine("settings warp gave some of, each said as it's set",
        policed?.parts.find(part => part.id === "settings"),
        Object.assign(figure("settings", "tunnel only", "split tunnel", "tunnel_only"),
            { parts: [
                figure("settings:mode", "tunnel only",
                    "the tunnel alone \u00b7 set by someone else", "tunnel_only"),
                figure("settings:always-on", "off", "", "off"),
                figure("settings:split-tunnel", "exclude", "1 host", "exclude")
            ] }));
    // An organisation disconnecting every device says so right after
    // the state, warning; an override in force says how long it has.
    const emergency = warpTold(Object.assign({}, warp, {
        settings: Object.assign({}, warp.settings, { emergency_disconnect: true }),
        override: { set: true, ends_in_secs: 700 } }));
    machine("an emergency disconnect said after the state, warning",
        emergency?.parts[1], Object.assign(figure("emergency", "on",
            "every device disconnected", "on"), { warns: true }));
    machine("an override in force, with the minutes it has left",
        emergency?.parts.find(part => part.id === "settings").parts
            .find(part => part.id === "settings:override"),
        figure("settings:override", "on", "12 min left", "on"));
    machine("a DNS proxy that's off says so in its place",
        policed?.parts.find(part => part.id === "dns")?.side, "proxy off");
    // Not answering, its note says what warp-cli's error means, and the
    // error as written is kept for the drawer (VKCF3Z, 2026-11-09).
    const notAnswering = said => Warp.failed({ error: "not-answering", said }, warpSaid);
    machine("WARP's service not answering, with no socket to reach",
        notAnswering("No such file or directory (os error 2)"), { headline: "not answering",
            warns: true, note: "its service isn't running",
            error: "No such file or directory (os error 2)" });
    machine("WARP's service not answering, its socket refusing",
        notAnswering("Connection refused (os error 111)").note, "its service isn't running");
    machine("WARP's service not answering, its socket shut to this account",
        notAnswering("Permission denied (os error 13)").note, "its service won't let you in");
    machine("WARP's service not answering in time",
        notAnswering("warp-cli status didn't answer within 5 s").note,
        "its service is too slow to answer");
    machine("WARP's service not answering for a reason warp has no words for",
        notAnswering("Address not available (os error 99)"), { headline: "not answering",
            warns: true, note: "its service can't be reached",
            error: "Address not available (os error 99)" });
    machine("warp-cli saying what warp can't read", Warp.failed({ error: "unreadable",
        said: "warp-cli status said InvalidAccountType: Invalid account type." }, warpSaid),
    { headline: "unreadable", warns: true,
        note: "warp-cli status said InvalidAccountType: Invalid account type." });
    machine("no warp-cli, so no WARP", Warp.failed({ error: "not-installed", said: "warp-cli" },
        warpSaid), null);
    machine("warp-cli's words as said", Warp.spoken("NetworkHealthy"), "network healthy");
    // Before the link is first read, its row says how WARP stands, as the
    // margin hears it all the time (modules/warp/src/serve.rs, warp:state),
    // so Network never opens on "\u2026" for a link it already knows.
    const standing = said => Warp.standing({ key: "warp:state", said }, warpSaid);
    machine("WARP connected, before its round trip is read",
        standing({ status: "Connected", reason: "NetworkHealthy" }),
        { headline: "connected", warns: false });
    machine("WARP switched off, as warp-cli says it",
        standing({ status: "Disconnected", reason: "Manual" }),
        { headline: "disconnected", warns: false });
    machine("WARP finding the network degraded warns",
        standing({ status: "Connected", reason: "NetworkDegraded" }),
        { headline: "degraded", warns: true });
    machine("WARP's service not answering warns", standing({ error: "FailedToConnectToDaemon" }),
        { headline: "not answering", warns: true });
    machine("warp-cli saying what isn't read leaves it waiting",
        standing({ error: "NothingWritten" }), null);
    machine("nothing heard of WARP leaves it waiting", Warp.standing(null, warpSaid), null);
    // What waits on the way through it, as the shaper heard it
    // (modules/warp/src/waiting.rs): past the way at rest, the least of
    // five minutes, after the Round Trip it explains.
    const waiting = shaped => WarpWaiting.part(shaped, {}, warpSaid);
    machine("what waits on the way is past the way at rest, noted both",
        waiting({ running: true, through_ms: 103.12174, at_rest_ms: 27.66719 }),
        figure("waiting", "75 ms", "103 ms through it \u00b7 28 ms at rest", "75"));
    machine("a wait within the way's own jitter is none",
        waiting({ running: true, through_ms: 31.2, at_rest_ms: 27.7 })?.side, "none");
    machine("the shaper off leaves it unknown, never none",
        waiting({ running: false }),
        figure("waiting", "unknown", "the shaper isn't running", ""));
    machine("a question through it unanswered leaves it unknown",
        waiting({ running: true, through_ms: null, at_rest_ms: 27.5 }),
        figure("waiting", "unknown", "its last question went unanswered", ""));
    machine("no shaper read says nothing of waiting", waiting(undefined), null);
    // The way judged slow by the warp daemon (modules/warp/src/slow.rs),
    // which the row says in place of how WARP stands (VKCF3Z, 2026-11-12).
    const slowed = (said, slow) =>
        WarpWaiting.slowed({ key: "warp:state", said, slow }, {}, warpSaid);
    const healthy = { status: "Connected", reason: "NetworkHealthy" };
    machine("a way judged slow says what waits, and not that it's connected",
        slowed(healthy, { waiting_ms: 120 }), "slow \u00b7 120 ms waiting");
    machine("a way that isn't slow leaves the row as WARP stands", slowed(healthy, null), null);
    machine("degraded says more than slow",
        slowed({ status: "Connected", reason: "NetworkDegraded" }, { waiting_ms: 120 }), null);
    machine("nor is a way down slow",
        slowed({ status: "Disconnected", reason: "NoNetwork" }, { waiting_ms: 120 }), null);
    const shapedParts = WarpWaiting.among(linked.parts,
        { running: true, through_ms: 103, at_rest_ms: 28 }, {}, warpSaid)
        .map(each => each.id);
    machine("Waiting follows the Round Trip",
        shapedParts.slice(shapedParts.indexOf("latency"), shapedParts.indexOf("latency") + 2),
        ["latency", "waiting"]);
    machine("with no tunnel timed, Waiting comes before Through",
        WarpWaiting.among([figure("state", "connected"), figure("through", "WARP")],
            { running: false }, {}, warpSaid).map(each => each.id),
        ["state", "waiting", "through"]);
    // The Work Network's own drawer, on its row in Network: whose link it
    // is, a lead saying how it stands and for how long, the facts that say
    // why, and warp-cli's own words last (VKCF3Z, 2026-11-12).
    const hour = 3_600_000;
    const stood = (status, reason, since) => ({ key: "warp:state", kind: "warp-state",
        read_at_ms: 10 * hour, since_ms: since, said: { status, reason } });
    const shapedLink = { parts: WarpWaiting.among(linked.parts,
        { running: true, through_ms: 103, at_rest_ms: 28 }, {}, warpSaid) };
    const drawerOf = (state, link = shapedLink) => WarpDrawer.drawer(
        { parts: link.parts, record: warp, stood: state, now: 13 * hour },
        { said: warpSaid, tells: vocabulary.find(thing => thing.id === "warp").tells });
    machine("the Work Network's drawer says whose, how it stands and why, then warp-cli's words",
        drawerOf(stood("Connected", "NetworkHealthy", 10 * hour)), {
            eyebrow: "via WARP \u00b7 tunnel",
            body: "Connected to work for 3 h, with a 127 ms round trip.",
            facts: [
                { id: "state", value: "connected" },
                { id: "latency", value: "127 ms" },
                { id: "waiting", value: "75 ms" },
                { id: "settings:mode", value: "warp+doh" },
                { id: "settings:organisation", value: "example" },
                { id: "through", value: "WARP" },
                { id: "status", value: "Connected", identifies: true },
                { id: "reason", value: "NetworkHealthy", identifies: true }
            ] });
    machine("a state first heard as the daemon started says no time, rather than too short",
        drawerOf(stood("Connected", "NetworkHealthy", null)).body,
        "Connected to work, with a 127 ms round trip.");
    machine("a link with no round trip read says only how long it's stood",
        drawerOf(stood("Connected", "NetworkHealthy", 12 * hour),
            { parts: [figure("state", "connected")] }).body,
        "Connected to work for 1 h.");
    machine("degraded, connecting and disconnected each lead as they stand", [
        drawerOf(stood("Connected", "NetworkDegraded", 12.5 * hour)).body,
        drawerOf(stood("Connecting", "", 13 * hour - 120_000)).body,
        drawerOf(stood("Disconnected", "Manual", 11 * hour)).body
    ], [
        "Connected to work, degraded for 30 min, with a 127 ms round trip.",
        "Connecting to work for 2 min.",
        "Disconnected from work for 2 h."
    ]);
    const slowFor = (reason, since) => Object.assign(stood("Connected", reason, 10 * hour),
        { slow: { waiting_ms: 120, since_ms: since } });
    machine("slow leads with how long it's been slow and what waits, in place of the trip",
        drawerOf(slowFor("NetworkHealthy", 13 * hour - 240_000)).body,
        "Connected to work, slow for 4 min, with 120 ms waiting.");
    machine("degraded says more than slow in the lead too",
        drawerOf(slowFor("NetworkDegraded", 13 * hour - 240_000)).body,
        "Connected to work, degraded for 3 h, with a 127 ms round trip.");
    machine("nothing heard of how it stands leaves the drawer no lead",
        drawerOf(null).body, "");
    // What the shell keeps of WARP's reads while it's shown, two seconds
    // apart, sparks the tunnel's Round Trip, Loss and Traffic, each with
    // a drawer saying once whose figures they are (VKCF3Z, 2026-11-08).
    const warpRead = (latency, loss, received, sent) => Object.assign({}, warp, {
        tunnel: Object.assign({}, warp.tunnel, { latency_ms: latency, loss_fraction: loss,
            received_bytes: received, sent_bytes: sent }) });
    const reads = [warpRead(120, 0.01, 1000, 500), warpRead(140, 0.02, 3000, 1500),
        warpRead(130, 0.015, 7000, 2500)];
    const seen = reads.reduce((kept, each, at) => WarpSeen.saw(kept, each, at * 2000), null);
    machine("WARP keeps what it read while shown, and the speed each way between",
        [seen.latency, seen.loss, seen.inward, seen.outward],
        [[120, 140, 130], [0.01, 0.02, 0.015], [1000, 2000], [500, 500]]);
    const spark = values => ({ shape: "spark", values, warn: false });
    const over = (values, least) => values.map(value => value / Math.max(least, ...values));
    const sparked = WarpSeen.drawn(Warp.told(reads[2], {}, warpSaid).parts, seen, {}, warpSaid);
    const drawn = id => (sparked.find(part => part.id === id) ?? {});
    const tunnel = "via WARP \u00b7 tunnel";
    const trip = spark(over([120, 140, 130], 100));
    machine("Round Trip sparks what was read, its drawer saying whose",
        [drawn("latency").graph, drawn("latency").drawer], [trip, { eyebrow: tunnel, facts: [
            { id: "range", value: "120 ms to 140 ms", figure: trip },
            { id: "reads", value: "3 over 4 s" }] }]);
    // A range whose low and high read the same at three digits says it's
    // steady, as a run, rather than "127 ms to 127 ms" (VKCF3Z, 2026-11-08).
    const still = [warpRead(127, 0.01, 1000, 500), warpRead(127.2, 0.01, 2000, 1000)];
    const steadily = still.reduce((kept, each, at) => WarpSeen.saw(kept, each, at * 2000), null);
    const steady = WarpSeen.drawn(Warp.told(still[1], {}, warpSaid).parts, steadily, {}, warpSaid)
        .find(part => part.id === "latency") ?? {};
    machine("a Round Trip that reads the same throughout says it's steady",
        steady.drawer?.facts?.[0]?.value, "127 ms \u00b7 steady");
    const lost = spark(over([0.01, 0.02, 0.015], 0.05));
    machine("Loss sparks against five percent at least, so a calm link reads small",
        [drawn("loss").graph, drawn("loss").drawer], [lost, { eyebrow: tunnel, facts: [
            { id: "range", value: "1.00% to 2.00%", figure: lost },
            { id: "reads", value: "3 over 4 s" }] }]);
    machine("Traffic sparks both ways together, its drawer the speed each way now",
        [drawn("traffic").graph, drawn("traffic").drawer], [spark(over([1500, 2500], 0)), {
            eyebrow: tunnel, facts: [
                { id: "in", value: "16.0 kb/s", figure: spark(over([1000, 2000], 0)) },
                { id: "out", value: "4.00 kb/s", figure: spark(over([500, 500], 0)) }] }]);
    machine("one read sparks nothing, so no drawer says only the row again",
        WarpSeen.drawn(Warp.told(reads[0], {}, warpSaid).parts, WarpSeen.saw(null, reads[0], 0),
            {}, warpSaid).filter(part => part.graph !== undefined || part.drawer !== undefined),
        []);
    const anew = WarpSeen.saw(seen, { status: { state: "Disconnected", reason: "Manual" },
        tunnel: null }, 6000);
    machine("a state changed starts the reads again",
        [anew.latency, anew.loss, anew.inward], [[], [], []]);
    const back = WarpSeen.saw(seen, warpRead(125, 0.01, 10, 5), 6000);
    machine("a counter gone back, as a link made again starts it, gives no speed",
        [back.latency.length, back.inward], [4, [1000, 2000]]);
    // WARP's switch, :connect and :disconnect: who holds it, where the
    // organisation does, and when WARP always on comes back, as warp's
    // own switch says it (modules/warp/src/switch.rs).
    const locked = Object.assign({}, warp, {
        settings: Object.assign({}, warp.settings, { switch_locked: true }) });
    machine("WARP's switch says when, always on, it comes back, who holds it, or nothing",
        [WarpSwitch.switchNote(warp, true, warpSaid),
            WarpSwitch.switchNote(locked, true, warpSaid),
            WarpSwitch.switchNote(warp, false, warpSaid)],
        ["comes back by itself in 2 min", "example holds the switch", ""]);
    // A switch the organisation holds draws no chord, since the foot
    // teaches only what works; Return stays and is refused with why
    // (VKCF3Z, 2026-11-08).
    machine("WARP's switch ends the link while up, unless the organisation holds it",
        [WarpSwitch.ends(warp, true), WarpSwitch.ends(locked, true),
            WarpSwitch.ends(warp, false)], [true, false, false]);
    machine("WARP's switch names who holds it, so it ends nothing, or nobody",
        [WarpSwitch.holder(locked), WarpSwitch.holder(warp)], ["example", ""]);
    machine("WARP up or on its way, so its switch takes it down",
        ["Connected", "Connecting", "Disconnected"].map(state =>
            WarpSwitch.up({ status: { state } })), [true, true, false]);
    machine("a switch done, held, refused or unread, as warp says each", [
        WarpSwitch.switched({ act: "disconnect", done: true, refused: null,
            comes_back_after_min: 2 }, warpSaid),
        WarpSwitch.switched({ act: "disconnect", done: false, refused: "held",
            organisation: "example" }, warpSaid),
        WarpSwitch.switched({ act: "connect", done: false, refused: "refused",
            said: "warp-cli wouldn't connect: Invalid account type." }, warpSaid),
        WarpSwitch.switched({ error: "not-answering", said: "its service didn't answer" },
            warpSaid)
    ], [
        { refused: "", note: "comes back by itself in 2 min" },
        { refused: "example holds the switch", note: "" },
        { refused: "warp-cli wouldn't connect: Invalid account type.", note: "" },
        { refused: "its service didn't answer", note: "" }
    ]);
    return asked;
}
