// What the launcher's WARP makes of what `warp --json` says (modules/warp,
// whose warp.json is the record's shape): how the link stands, in a few
// words for its row in Network, and each figure as a part, in its own
// unit, those of the settings, TLS and DNS held a step in. Every word is
// the vocabulary's (launcher/words.json, warp's `said`), handed in as
// `said`, and every figure is written as HomeNetwork.js writes one.
// Warp.qml runs warp and draws the rows; nothing here asks anything, so
// tests/answers.qml runs it in Qt's own engine over warp.json itself.
.pragma library
.import "HomeNetwork.js" as HomeNetwork
.import "Ago.js" as Ago

const AND = " \u00b7 ";
const PERCENT = 100;
const SECOND = 1000;
const MINUTE = 60;
// warp-cli's reasons that say how the network is, "Network" then a word.
const NETWORK = /^Network([A-Z].*)$/;
// A protocol given with how it's carried: "MASQUE (HTTPS via UDP)".
const CARRIED = /^(.*\S)\s*\((.+)\)$/;
const CAPITAL = /([a-z0-9])([A-Z])/g;
const UNDERSCORE = /_/g;
const PLACEHOLDER = /\{(\w+)\}/g;
// The unit a figure's key ends in, as warp-cli names them.
const UNIT = /^(.*)_(percent|ms)$/;
// What warp says when it couldn't read WARP, by its error, as the words
// its row says it in; a machine with no warp-cli has no WARP, so says
// nothing.
const FAILED = {
    "not-answering": "notAnswering",
    unreadable: "unreadable",
};
// What warp-cli's error means when WARP's service isn't answering, as the
// word its note says it in, first match winning. warp-cli passes on the
// system's error for the service's socket: none there (2) or nothing
// listening on it (111) is a service that isn't running, and one shut to
// this account (13) won't let it in. warp's own words for a warp-cli that
// hangs say it's too slow. Anything else can't be reached, and the error as
// written is in the drawer for whoever looks further (VKCF3Z, 2026-11-09).
const WHY = [
    [/\(os error (2|111)\)$/, "notRunning"],
    [/\(os error 13\)$/, "shutOut"],
    [/didn't answer within/, "tooSlow"],
];

// A word of warp's in the vocabulary, its {placeholders} filled.
function worded(said, key, fill) {
    return String(said?.[key] ?? "").replace(PLACEHOLDER, (whole, name) =>
        fill?.[name] === undefined ? whole : String(fill[name]));
}

// warp-cli's word as a person says it: "NetworkHealthy" as "network
// healthy".
function spoken(word) {
    return String(word).replace(CAPITAL, "$1 $2").replace(UNDERSCORE, " ").toLowerCase();
}

function text(value) {
    return typeof value === "string" && value !== "";
}

// Items said together, joined by dots, those with nothing to say giving
// way.
function joined(items) {
    return items.filter((item) => item !== "").join(AND);
}

// Milliseconds as WARP gives them: a whole one as it is, since WARP
// measures no finer, else to three digits, as every figure is.
function ms(value, settings, said) {
    const figure = Number.isInteger(value) ? HomeNetwork.number(value, 0, settings)
        : HomeNetwork.significant(value, settings);
    return worded(said, "ms", { figure });
}

function count(value, settings) {
    return HomeNetwork.number(value, 0, settings);
}

function onOff(value, said) {
    return worded(said, value ? "on" : "off");
}

// What a reason's figures say, by the unit each key ends in:
// { loss_percent: 5.37, rtt_ms: 170 } as "5.37% loss · 170 ms round trip".
function measured(figures, settings, said) {
    return joined(Object.keys(figures ?? {}).map((key) => {
        const unit = UNIT.exec(key);
        const value = Number(figures[key]);
        if (!(unit && Number.isFinite(value))) {
            return "";
        }
        const name = unit[1] === "rtt" ? worded(said, "rtt") : spoken(unit[1]);
        const figure = unit[2] === "percent"
            ? worded(said, "percent", { figure: HomeNetwork.significant(value, settings) })
            : ms(value, settings, said);
        return `${figure} ${name}`;
    }));
}

// A part saying a figure, copying it as given, warning of nothing unless
// it's said to.
function part(id, side, note, copy) {
    return { id, side, note: note ?? "", copy: copy ?? side, warns: false };
}

// A part holding parts of its own, a step in, or null where it holds
// none.
function holding(figure, parts) {
    return parts.length === 0 ? null : Object.assign(figure, { parts });
}

// The parts given, leaving out those that aren't.
function present(parts) {
    return parts.filter((each) => each !== null);
}

// The network's health as WARP judges it, from a reason that says so, or
// null: its state alone at the side, since a state never gives way, and
// what WARP measured it by in its note, whose measure it is first, so
// that gives way last: the tunnel's own Round Trip and Loss are its
// estimates, and two round trips unnamed would read as a contradiction
// (VKCF3Z, 2026-11-08): "WARP's measure \u00b7 5.37% loss \u00b7 170 ms round
// trip".
function health(status, settings, said) {
    const judged = NETWORK.exec(status.reason ?? "");
    if (!judged) {
        return null;
    }
    const side = spoken(judged[1]);
    const figures = measured(status.reason_figures, settings, said);
    const note = figures === "" ? "" : joined([worded(said, "measure"), figures]);
    return Object.assign(part("health", side, note), { warns: side !== "healthy" });
}

// Who set a setting, in warp-cli's words (`network_policy`), as its note
// says it, so a setting Joel can't change says whose it is; nothing where
// warp-cli didn't say.
function setBy(shaped, key, said) {
    const who = shaped.set_by?.[key];
    if (!text(who)) {
        return "";
    }
    const known = worded(said, `by:${who}`);
    return known === "" ? worded(said, "by", { who: spoken(who) }) : known;
}

// An entry of a list, a host or an address, named by itself, what's said
// of it beside it, and copied as it's named.
function entry(id, name, side) {
    return { id, name: String(name), side: side ?? "", note: "", copy: String(name), warns: false };
}

// A list warp gave, or none.
function listOf(value) {
    return Array.isArray(value) ? value : [];
}

// Seconds as the whole minutes they reach, so a minute begun counts.
function minutesOf(seconds, settings, said) {
    return worded(said, "minutes", { figure: count(Math.ceil(seconds / MINUTE), settings) });
}

// The split tunnel's lists by how many each holds, one that's empty
// giving way: "2 hosts · 2 networks".
function listed(split, settings, said) {
    const many = (value, one, several) => (value > 0
        ? worded(said, value === 1 ? one : several, { count: count(value, settings) }) : "");
    return [many(split.hosts, "host", "hosts"), many(split.ips, "network", "networks")];
}

// The settings that shape the link, each only where warp gave it, with
// who set it: Mode first, saying what it does, then Always On and the
// Split Tunnel, the three that decide what reaches the tunnel, then the
// rest. The part holding them says the mode, and whether it's always on
// and splits, so the three are read without a step in.
function settingsPart(shaped, override, settings, said) {
    const parts = [];
    // A setting by its id, its key in the record, and what it shows:
    // { side, note, copy }, who set it said after its note.
    const add = (id, key, shown) => parts.push(part(`settings:${id}`, shown.side,
        joined([shown.note ?? "", setBy(shaped, key, said)]), shown.copy));
    const asGiven = (value) => ({ side: spoken(value), copy: value });
    if (text(shaped.mode)) {
        add("mode", "mode", Object.assign(asGiven(shaped.mode),
            { note: worded(said, `mode:${shaped.mode}`) }));
    }
    if (typeof shaped.always_on === "boolean") {
        add("always-on", "always_on", { side: onOff(shaped.always_on, said) });
    }
    const split = shaped.split_tunnel;
    if (split && text(split.mode)) {
        add("split-tunnel", "split_tunnel", Object.assign(asGiven(split.mode),
            { note: joined(listed(split, settings, said)) }));
        // Each host and network it lists, a step in, with what it's for.
        const entries = listOf(split.host_list).map((one) =>
            entry("settings:split-tunnel:host", one.value,
                text(one.description) ? one.description : ""))
            .concat(listOf(split.ip_list).map((one) => entry("settings:split-tunnel:network",
                one.value, text(one.description) ? one.description : "")));
        holding(parts[parts.length - 1], entries);
    }
    if (typeof shaped.switch_locked === "boolean") {
        add("switch-locked", "switch_locked", { side: onOff(shaped.switch_locked, said) });
    }
    const minutes = shaped.auto_connect_after_min;
    if (Number.isFinite(minutes)) {
        add("auto-connect", "auto_connect_after_min", {
            side: worded(said, "minutes", { figure: count(minutes, settings) }),
            copy: String(minutes),
        });
    }
    if (text(shaped.organisation)) {
        add("organisation", "organisation", { side: shaped.organisation });
    }
    if (text(shaped.tunnel_protocol)) {
        add("tunnel-protocol", "tunnel_protocol", asGiven(shaped.tunnel_protocol));
    }
    if (text(shaped.post_quantum)) {
        add("post-quantum", "post_quantum", asGiven(shaped.post_quantum));
    }
    parts.push(...present(lookupParts(shaped, settings, said)),
        ...present(allowedParts(shaped, override, settings, said)));
    const mode = text(shaped.mode) ? shaped.mode : "";
    const splits = split && split.hosts + split.ips > 0;
    return holding(part("settings", spoken(mode), joined([
        shaped.always_on === true ? worded(said, "alwaysOn") : "",
        splits ? worded(said, "split") : "",
    ]), mode), parts);
}

// Where names are looked up: the DoH server, its addresses a step in, and
// the domains looked up on the network's own servers instead, each a
// step in with the servers it's asked of; then how a sign-in page is met
// and whether the way's largest packet is found.
function lookupParts(shaped, settings, said) {
    const resolvers = listOf(shaped.doh_resolver_ips);
    const domains = listOf(shaped.fallback_domains);
    const many = (value, one, several) => worded(said, value === 1 ? one : several,
        { count: count(value, settings) });
    const portal = shaped.captive_portal ?? {};
    const opens = portal.firewall_off_s;
    return [
        text(shaped.doh_hostname)
            ? Object.assign(part("settings:doh", shaped.doh_hostname,
                resolvers.length > 0 ? many(resolvers.length, "address", "addresses") : ""),
            resolvers.length > 0 ? { parts: resolvers.map((address) =>
                entry("settings:doh:address", address)) } : {})
            : null,
        holding(part("settings:local-domains", many(domains.length, "domain", "domains"), "",
            String(domains.length)), domains.map((one) => entry("settings:local-domains:domain",
            one.domain, joined(listOf(one.dns_servers))))),
        typeof portal.detection === "boolean"
            ? part("settings:sign-in-pages", onOff(portal.detection, said),
                portal.detection && opens > 0
                    ? worded(said, "firewallOpens", { minutes: minutesOf(opens, settings, said) })
                    : "")
            : null,
        typeof shaped.enable_pmtud === "boolean"
            ? part("settings:path-mtu", onOff(shaped.enable_pmtud, said)) : null,
    ];
}

// What the organisation lets this machine do: switch its mode, leave it,
// and ask for help where; how large a local network an override reaches,
// and the override itself, with how long it has left while it's in force.
function allowedParts(shaped, override, settings, said) {
    const prefix = shaped.lan_subnet_size;
    const left = override?.ends_in_secs;
    return [
        typeof shaped.allow_mode_switch === "boolean"
            ? part("settings:mode-switch", onOff(shaped.allow_mode_switch, said)) : null,
        typeof shaped.allowed_to_leave === "boolean"
            ? part("settings:may-leave", onOff(shaped.allowed_to_leave, said)) : null,
        text(shaped.support_url) ? part("settings:support", shaped.support_url) : null,
        Number.isFinite(prefix)
            ? part("settings:local-network", worded(said, "prefix", { size: prefix }), "",
                String(prefix))
            : null,
        typeof override?.set === "boolean"
            ? part("settings:override", onOff(override.set, said), override.set && left > 0
                ? worded(said, "left", { minutes: minutesOf(left, settings, said) }) : "")
            : null,
    ];
}

// The tunnel's TLS, each figure only where warp gave it, the part holding
// them saying its version and whether it's post-quantum.
function tlsPart(tls, said) {
    const parts = [];
    for (const key of ["version", "curve", "cipher"]) {
        if (text(tls[key])) {
            parts.push(part(`tls:${key}`, tls[key]));
        }
    }
    if (typeof tls.post_quantum === "boolean") {
        parts.push(part("tls:post-quantum", onOff(tls.post_quantum, said)));
    }
    if (text(tls.sni)) {
        parts.push(part("tls:server-name", tls.sni));
    }
    const version = text(tls.version) ? tls.version : "";
    return holding(part("tls", version,
        tls.post_quantum === true ? worded(said, "postQuantum") : ""), parts);
}

// What WARP's DNS proxy has answered on this connection, each count only
// where warp gave it, the part holding them saying its average and how
// many it answered, or that the proxy is off.
function dnsPart(dns, settings, said) {
    const parts = [];
    if (typeof dns.proxy_enabled === "boolean") {
        parts.push(part("dns:proxy", onOff(dns.proxy_enabled, said)));
    }
    const average = Number.isFinite(dns.average_ms);
    if (average) {
        parts.push(part("dns:average", ms(dns.average_ms, settings, said), "",
            String(dns.average_ms)));
    }
    const counts = [["asked", "total"], ["answered", "success"], ["no-record", "no_records_found"],
        ["timed-out", "timed_out"], ["failed", "other_error"]];
    for (const [id, key] of counts) {
        if (Number.isFinite(dns[key])) {
            parts.push(part(`dns:${id}`, count(dns[key], settings), "", String(dns[key])));
        }
    }
    let side = average ? ms(dns.average_ms, settings, said) : "";
    if (dns.proxy_enabled === false) {
        side = worded(said, "proxyOff");
    }
    const answered = dns.total > 0 && Number.isFinite(dns.success)
        ? worded(said, "answered", {
            success: count(dns.success, settings),
            total: count(dns.total, settings),
        })
        : "";
    return holding(part("dns", side, answered, average ? String(dns.average_ms) : ""), parts);
}

// The tunnel's endpoint, its IPv4 address where it has one, its note
// the IPv6 one, or saying there's none of the other.
function endpoint(tunnel, said) {
    const v4 = text(tunnel.v4_endpoint) ? tunnel.v4_endpoint : "";
    const v6 = text(tunnel.v6_endpoint) ? tunnel.v6_endpoint : "";
    if (v4 === "" && v6 === "") {
        return null;
    }
    if (v4 === "") {
        return part("endpoint", v6, worded(said, "noIpv4"));
    }
    return part("endpoint", v4, v6 === "" ? worded(said, "noIpv6") : v6);
}

// What the tunnel has carried each way, as Wi-Fi's Speed says its rates:
// "10.2 GB in \u00b7 9.26 GB out", a way warp didn't give leaving the other
// alone; or null where it gave neither.
function trafficPart(tunnel, settings, said) {
    const way = (key, word) => (Number.isFinite(tunnel[key])
        ? worded(said, word, { figure: HomeNetwork.bytes(tunnel[key], settings) }) : "");
    const side = joined([way("received_bytes", "in"), way("sent_bytes", "out")]);
    return side === "" ? null : part("traffic", side);
}

// The tunnel's parts, each only where warp gave its figure. That they
// are WARP's own, through its tunnel, is its drawer's eyebrow's to say
// once ("via WARP \u00b7 tunnel"), rather than each part's note (VKCF3Z,
// 2026-11-08), so a note says only what differs. What the record holds
// and no part says is only this, each for its reason:
// - `tunnel.on` while it's true, since State already says the link is up;
// - a reason's figure in a unit other than percent or milliseconds, which
//   warp-cli hasn't been seen to give, so there's no unit to say it in.
// The history the record doesn't keep, to spark latency, loss and traffic,
// is the shell's, kept while WARP is shown (WarpSeen.js).
// What warp-cli gives that the record leaves out, and why, is in
// modules/warp/contract/README.md.
function tunnelParts(tunnel, settings, said) {
    const finite = (key, made) => (Number.isFinite(tunnel[key]) ? made(tunnel[key]) : null);
    return present([
        tunnel.on === false ? part("tunnel", worded(said, "off")) : null,
        finite("latency_ms", (value) => part("latency", ms(value, settings, said), "",
            String(value))),
        finite("loss_fraction", (value) => part("loss", worded(said, "percent",
            { figure: HomeNetwork.significant(value * PERCENT, settings) }), "", String(value))),
        protocolPart(tunnel.protocol),
        coloPart(tunnel.edge, said),
        endpoint(tunnel, said),
        finite("last_handshake_s", (value) => part("handshake", Ago.ago(0, value * SECOND), "",
            String(value))),
        trafficPart(tunnel, settings, said),
        tunnel.tls ? tlsPart(tunnel.tls, said) : null,
    ]);
}

// The tunnel's protocol, how it's carried its note, or null.
function protocolPart(protocol) {
    if (!text(protocol)) {
        return null;
    }
    const carried = CARRIED.exec(protocol);
    return carried
        ? part("protocol", carried[1], carried[2], protocol)
        : part("protocol", protocol);
}

// The data centre the tunnel ends in, the server there its note, or null.
function coloPart(edge, said) {
    const colo = edge?.colo ?? "";
    if (colo === "") {
        return null;
    }
    const metal = edge.metal ?? "";
    return part("colo", colo, metal === "" ? "" : worded(said, "metal", { metal }));
}

// How WARP stands, from what `warp --json` printed: { headline, warns,
// parts }, each part { id, side, note, copy, warns } by its id in the
// vocabulary (warp:<id>), one holding more { parts } too, a step in; or
// null for a record with no status. The parts go how it stands, the
// settings, the tunnel, DNS, then what carries it, as `warp`'s own page
// has them, but for the settings coming first, as a thing's settings come
// before its readings. The headline is its state, or the network's health while
// connected, with the latency beside it where warp gave one: "connected
// · 127 ms". `settings` says how a figure is written (its group and
// point), as HomeNetwork.js takes it, and `said` is warp's words. The
// sparks of what's been read while it's shown are WarpSeen.js's.
function told(record, settings, said) {
    const status = record?.status;
    if (!status || typeof status.state !== "string") {
        return null;
    }
    const state = spoken(status.state);
    const judged = health(status, settings, said);
    // A reason that isn't the network's health says why it stands so.
    const why = !judged && status.reason ? spoken(status.reason) : "";
    const parts = [part("state", state, why)];
    // An organisation disconnecting every device at once is said first.
    const emergency = record.settings?.emergency_disconnect === true
        ? Object.assign(part("emergency", worded(said, "on"), worded(said, "emergency")),
            { warns: true })
        : null;
    parts.push(...present([
        emergency,
        judged,
        record.settings
            ? settingsPart(record.settings, record.override ?? null, settings, said) : null,
        ...(record.tunnel
            ? tunnelParts(record.tunnel, settings, said) : []),
        record.dns ? dnsPart(record.dns, settings, said) : null,
        // The place is the work network, Joel's word for it (2026-11-11),
        // so what carries it is named once, last (VKCF3Z, 2026-11-11).
        part("through", worded(said, "through")),
    ]));
    const warns = judged?.warns === true;
    const first = warns ? judged.side : state;
    const latency = parts.find((each) => each.id === "latency");
    return {
        headline: latency ? `${first}${AND}${latency.side}` : first,
        warns,
        parts,
    };
}

// What warp said when it couldn't read WARP ({ error, said }), as the row
// says it in warp's words (`said`): { headline, warns, note }, or null
// where there's no warp-cli, so no WARP to show. Its note is why, so the
// row reads as one run, "not answering \u00b7 its service isn't running",
// rather than saying it isn't answering twice. Not answering, the note
// says what warp-cli's error means and `error` keeps it as written, for
// the drawer; unreadable, the note is what warp couldn't read.
function failed(printed, said) {
    const word = FAILED[printed?.error];
    if (!word) {
        return null;
    }
    const error = String(printed.said ?? "");
    if (printed.error !== "not-answering") {
        return { headline: worded(said, word), warns: true, note: error };
    }
    const why = WHY.find(([means]) => means.test(error));
    return {
        headline: worded(said, word),
        warns: true,
        note: worded(said, why ? why[1] : "unreached"),
        error,
    };
}
