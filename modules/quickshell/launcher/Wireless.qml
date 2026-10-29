// Wi-Fi, a collection: the networks in the air, the one connected first,
// then those saved, then the rest by signal, each glyph saying how strong
// it is and whether it's locked, the footer how it's connected. Return
// disconnects the one connected or connects another, and the launcher stays,
// saying how far it has got, until the network takes the card or says why
// not, in the words Bluetooth's devices use, since it's the same act; a locked
// network met for the first time, or one whose password didn't work, asks
// for its password in the field beside the padlock, and one asking for an
// account asks its name, its server's domain and its password in turn, the
// server proving it's that domain's before it's told. Stepping into a
// network gives, after what can be done to it, the network in detail, each
// a part named first: the card's Signal (a spark of it while shown, each
// antenna, the transmit power a step in), its Speed (each way, the Wi-Fi
// generation, coding, width, channel and Retries a step in), its
// Protection, said for what it means, its Access points, each a step into
// its own parts, with the one wifid would pick and why, and how long it's
// been Connected for. A network wanting a sign-in offers to sign in to it
// first. Turning Wi-Fi off and on is an action in the main search.
// Everything else comes from wifid over its socket (Wifi.qml), and the
// cards look for networks only while the launcher is open. What wifid
// doesn't say of a link (each antenna, transmit power, retries, how long
// it's been on, the generation) iw reads, every few seconds while the
// network joined is shown, by one spawn of the `iw` setting's script, and
// never for a key.
import qs
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Interfaces.js" as Interfaces
import "Numbers.js" as Numbers

Provider {
    id: provider

    what: "Wi-Fi"

    // A machine without wifid, or without a card, has no Wi-Fi to show.
    readonly property bool present: Wifi.described && Wifi.cards.length > 0
    readonly property bool on: present && Wifi.radio

    collection: !present ? null : Words.collection("wifi", {
        empty: on ? "Looking for networks\u2026" : "Wi-Fi is off."
    })

    // Looking for networks costs the radio's power, so the cards look only
    // while the launcher is there to show what they hear.
    Timer {
        interval: 10000
        repeat: true
        triggeredOnStart: true
        running: provider.on && provider.launcher !== null && provider.launcher.open
        onTriggered: Wifi.scan()
    }

    readonly property var radio: thing({
        key: "wifi:radio",
        // An act, not a network, so it says no kind (docs/design.md).
        what: "",
        verb: Words.verb("wifi:radio", on ? "" : "off"),
        name: Words.name("wifi:radio", on ? "" : "off"),
        glyph: Words.glyph("wifi:radio", on ? "" : "off"),
        hue: "machine",
        stays: true,
        run: () => Wifi.turn(!provider.on)
    })

    things: !present ? [] : SignIn.network !== "" ? [signIn, radio] : [radio]

    // While a network wants a sign-in, it's what's next.
    readonly property var signIn: thing({
        key: "wifi:sign-in",
        what: "",
        verb: Words.verb("wifi:sign-in"),
        name: Words.name("wifi:sign-in", "", { network: SignIn.network }),
        glyph: Words.glyph("wifi:sign-in"),
        hue: "machine",
        run: () => SignIn.open()
    })

    // How a network is joined, in words, and what's typed here to open it:
    // a password, or an account with its server and password.
    function security(network) {
        switch (network.security) {
        case "open":
            return { words: "open", locked: false };
        case "personal":
            return { words: "a password", locked: true, password: true };
        default:
            return { words: "an account", locked: true, password: true, account: true };
        }
    }

    // Signal in four bars, as the theme's marks draw it: with and without a
    // lock, and an outline for none heard.
    function strengthGlyph(bars, locked) {
        const kind = locked ? "lockedSignal" : "signal";
        return Theme.glyph[`${kind}${Math.max(0, Math.min(4, bars))}`];
    }

    // Every network in the air, and the one joined even when it hides its
    // name, each with what the rows need to know of it.
    readonly property var networks: {
        if (!on)
            return [];
        const heard = Wifi.nearby.map(network => described(network.name, network.security,
            network.strength));
        for (const card of Wifi.cards) {
            const kept = Wifi.isSaved(card.network ?? "");
            if (card.network && !heard.some(network => network.name === card.network))
                heard.push(described(card.network, kept ? kept.security : "open", 0));
        }
        const rank = network => network.connected ? 0 : network.known ? 1 : 2;
        return heard.sort((a, b) => rank(a) - rank(b) || b.strength - a.strength
            || a.name.localeCompare(b.name));
    }

    function described(name, security, strength) {
        const kept = Wifi.isSaved(name);
        return {
            name: name,
            security: security,
            strength: strength,
            known: kept !== null,
            declared: kept !== null && kept.declared,
            trusted: kept !== null && kept.trusted,
            connected: Wifi.isJoined(name)
        };
    }

    // What the last join said if it couldn't, and of which network.
    property string troubled: ""
    property string trouble: ""

    function join(network, password, account) {
        troubled = "";
        trouble = "";
        Wifi.join(network.name, password, account);
    }

    // A network asking for an account, met for the first time, asks the
    // account's name, then the domain its server proves it's the network's
    // with, then the password, each in the field in turn. PEAP is how most
    // are joined; TTLS is a verb.
    function accountFor(network, method) {
        return thing({
            key: `wifi:${network.name}:identity`,
            verb: "next",
            name: network.name,
            chip: network.name,
            ask: "the account's name",
            network: network.name,
            state: () => provider.said(network, `an account, over ${method.toUpperCase()}`),
            given: identity => provider.launcher.ask(provider.serverFor(network, method, identity))
        });
    }

    function serverFor(network, method, identity) {
        return thing({
            key: `wifi:${network.name}:server`,
            verb: "next",
            name: network.name,
            chip: identity,
            ask: "its server's domain",
            network: network.name,
            state: () => provider.said(network, "the domain its server's certificate names"),
            given: domain => provider.launcher.ask(provider.passwordFor(network, {
                method: method,
                identity: identity,
                serverDomain: domain
            }))
        });
    }

    function passwordFor(network, account) {
        return thing({
            key: `wifi:${network.name}:account-password`,
            verb: "connect",
            name: network.name,
            chip: account.serverDomain,
            ask: "password",
            secret: true,
            network: network.name,
            state: () => provider.said(network, `${account.identity}'s password`),
            given: password => provider.join(network, password, account)
        });
    }

    // How far a join has got, in the footer's words.
    readonly property var stages: ({
        joining: "connecting to",
        gettingAddress: "getting an address from",
        checking: "seeing how far it reaches:"
    })

    // What the footer says while a network is joined, or wouldn't be.
    function said(network, otherwise) {
        if (Wifi.joining && Wifi.joining.name === network.name)
            return `${stages[Wifi.joining.stage] ?? stages.joining} ${network.name}\u2026`;
        if (troubled === network.name && trouble !== "")
            return trouble;
        return otherwise;
    }

    // The field asking for a network's password, the padlock beside it.
    function asking(network, why) {
        const locked = security(network);
        return thing({
            key: `wifi:${network.name}:password`,
            verb: "connect",
            name: network.name,
            chip: network.name,
            ask: "password",
            secret: true,
            network: network.name,
            state: () => provider.said(network, why),
            given: password => provider.join(network, password),
            note: locked.words
        });
    }

    function made(network) {
        const locked = security(network);
        const joined = network.connected;
        // Met for the first time behind a password, it asks for one; behind
        // an account, for the account first.
        const met = !joined && !network.known;
        const asks = met && locked.password === true && !locked.account;
        const signal = network.strength > 0 ? `${network.strength} of 4 bars · ` : "";
        const note = `${signal}${locked.words}`;
        const opening = met && locked.account ? provider.accountFor(network, "peap") : null;
        // Joined but wanting a sign-in, Return signs in, as it's all that's
        // left to do there; otherwise it leaves a network joined or joins one.
        const signing = joined && SignIn.network === network.name;
        return member({
            key: `wifi:${network.name}`,
            verb: signing ? Words.verb("verb:wifi-sign-in") : joined ? "disconnect" : "connect",
            name: network.name,
            glyph: strengthGlyph(network.strength, locked.locked),
            hue: "machine",
            // Connected, as Return connects it and its access point says it
            // is; join is a call's word (docs/glossary.md).
            side: joined ? "connected" : network.known ? "saved" : "",
            note: note,
            network: network.name,
            state: () => provider.said(network, note),
            // Joining or leaving keeps the launcher, which says how it went;
            // signing in opens the page, so it closes.
            stays: !signing,
            secret: asks,
            ask: asks ? "password" : opening ? opening.ask : "",
            chip: network.name,
            given: asks ? password => provider.join(network, password)
                : opening ? opening.given : null,
            run: signing ? () => SignIn.open() : joined ? () => Wifi.leave(network.name)
                : () => provider.join(network),
            verbs: () => provider.verbs(network, locked)
        });
    }

    readonly property var all: networks.map(network => made(network))
    onAllChanged: changed()

    // The network stepped into, asked about in detail for as long as it,
    // or its access points, are shown.
    readonly property string looking: {
        const launcher = provider.launcher;
        if (!launcher || !launcher.open)
            return "";
        const step = launcher.held.find(step => step.row.key === `wifi:${step.row.network}`);
        return step ? step.row.network : "";
    }
    onLookingChanged: {
        order = [];
        linkRead = null;
        signalSeen = [];
        Wifi.watch(looking);
    }

    // Its rows change as wifid hears more of it. Without iw to read the
    // signal, wifid's own reading draws its spark.
    Connections {
        target: Wifi

        function onDetailChanged() {
            if (provider.looking === "")
                return;
            const detail = Wifi.detail;
            if (provider.reader === "" && detail && detail.joined.length > 0
                && detail.joined[0].signal !== null)
                provider.saw(detail.joined[0].signal);
            provider.changed();
        }
    }

    // The card on the network shown, whose link iw reads while it's shown.
    readonly property string looked: looking === "" ? ""
        : (Wifi.cards.find(card => card.network === looking) ?? { name: "" }).name
    readonly property string reader: settings.iw ?? ""

    // What iw last read of that card's link (Interfaces.radio), as
    // { card, said }, and the signal it's read since the network was shown,
    // each from 0 to 1, the newest last.
    property var linkRead: null
    property var signalSeen: []
    readonly property int graphed: 20

    function saw(dbm) {
        signalSeen = signalSeen.concat([Interfaces.level(dbm)]).slice(-graphed);
    }

    Timer {
        interval: 3000
        repeat: true
        triggeredOnStart: true
        running: provider.looked !== "" && provider.reader !== ""
        onTriggered: {
            if (iw.running)
                return;
            iw.command = [provider.reader, provider.looked];
            iw.running = true;
        }
    }

    Process {
        id: iw
        stdout: StdioCollector {
            id: iwSaid
            onStreamFinished: provider.readOf(iw.command[1] ?? "", iwSaid.text)
        }
    }

    function readOf(card, text) {
        if (card !== looked)
            return;
        const said = Interfaces.radio(text);
        linkRead = said ? { card: card, said: said } : null;
        if (said && said.signal !== null)
            saw(said.signal);
        changed();
    }

    // The access points in the order they were first shown, fastest first,
    // so a row stays where it is as its readings change, and one heard
    // later joins the end.
    property var order: []

    // How many letters a footer has room for beside its keys.
    readonly property int room: 40

    readonly property var ghz: ({ ghz2: "2.4 GHz", ghz5: "5 GHz", ghz6: "6 GHz" })

    // The first of these, then as many more as fit, said together.
    function fitting(parts) {
        let said = "";
        for (const part of parts.filter(part => part !== "")) {
            const longer = said === "" ? part : `${said} \u00b7 ${part}`;
            if (said !== "" && longer.length > room)
                break;
            said = longer;
        }
        return said;
    }

    function number(value) {
        return Numbers.format(value, 0, settings.group, settings.point);
    }

    // A part's name, as the vocabulary says it: "Signal" for wifi:signal.
    function named(id, fill) {
        return Words.name(`wifi:${id}`, "", fill);
    }

    // A part saying a value, Return copying it, wearing no glyph as a
    // figure doesn't.
    function fact(key, id, side, fields) {
        return copyRow(Object.assign({
            key: key,
            name: named(id),
            side: side,
            copy: side,
            hue: "machine",
            glyph: "",
            note: ""
        }, fields ?? {}));
    }

    // A part holding more, which copies its value on Return and steps in
    // by the right arrow, wearing Wi-Fi's glyph dimmed. What it holds is made as it's opened,
    // from what wifid and iw say then, so it follows them while shown.
    function holds(key, id, side, fields, inside) {
        return holderRow(Object.assign({
            key: key,
            name: named(id),
            side: side,
            glyph: Theme.glyph.wireless
        }, fields ?? {}), inside);
    }

    function level(dbm) {
        return { shape: "bar", values: [Interfaces.level(dbm)], warn: false };
    }

    // The network shown in detail, as wifid last said it, or null.
    function detailOf(name) {
        const detail = Wifi.detail;
        return detail && detail.name === name && detail.points.length > 0 ? detail : null;
    }

    // A card's link as wifid says it, and as iw last read it, or null.
    function linkOf(name, card) {
        const detail = detailOf(name);
        return detail ? detail.joined.find(link => link.card === card) ?? null : null;
    }

    function readOfCard(card) {
        return linkRead && linkRead.card === card ? linkRead.said : null;
    }

    // A network in detail, after what can be done to it: the link of the
    // card on it (a Card each, while there are several), how it's
    // protected, and its access points. Return or the right arrow opens a
    // part holding more, and copies a figure.
    function facts(network) {
        const detail = detailOf(network.name);
        // A network joined waits for its card's link, so its parts arrive
        // together, in their order, rather than the link's below the rest.
        if (detail === null || (network.connected && detail.joined.length === 0))
            return [];
        const name = network.name;
        let rows = [];
        if (detail.joined.length === 1)
            rows = linkParts(name, detail.joined[0].card);
        else
            rows = detail.joined.map((link, at) => holds(`wifi:card:${link.card}`, "card",
                link.signal !== null ? `${link.signal} dBm` : "",
                { name: named("card", { n: at + 1 }), keywords: link.card },
                () => provider.linkParts(name, link.card)
                    .concat(provider.connectedRows(link.card))));
        rows.push(guardRow(name));
        rows.push(heardRow(name));
        // iw says how long it's been on a moment after the rest arrive, so
        // it comes last, where it lands whenever it's read.
        if (detail.joined.length === 1)
            rows = rows.concat(connectedRows(detail.joined[0].card));
        rows[0].gap = true;
        return rows;
    }

    // A card's link: its Signal, Speed and how long it's been Connected
    // for, each as the driver last read it.
    function linkParts(name, card) {
        const link = linkOf(name, card);
        if (link === null)
            return [];
        const read = readOfCard(card);
        const key = id => `wifi:link:${card}:${id}`;
        const signal = read?.signal ?? link.signal;
        const rows = [holds(key("signal"), "signal",
            signal !== null ? `${signal} dBm` : "not read yet", {
                keywords: card,
                graph: signalSeen.length > 0
                    ? { shape: "spark", values: signalSeen, warn: false } : undefined,
                note: link.margin !== null ? `${link.margin} dB over the noise`
                    : "while it's shown"
            }, () => provider.signalParts(name, card))];
        const rates = speeds(link, read);
        rows.push(holds(key("speed"), "speed", rates.side !== "" ? rates.side : "not read yet",
            { note: rates.note },
            () => provider.speedParts(name, card)));
        return rows;
    }

    // How long the card's been on the network, as iw says.
    function connectedRows(card) {
        const read = readOfCard(card);
        if (!read || read.connected === null)
            return [];
        const lasted = Interfaces.lasted(read.connected);
        return [fact(`wifi:link:${card}:connected`, "connected", lasted, {
            note: `since ${Qt.formatDateTime(new Date(Date.now() - read.connected * 1000),
                "HH:mm")}`
        })];
    }

    // The rates each way, as iw reads them or else as wifid does, in Mb/s,
    // and what's said of them at a glance.
    function speeds(link, read) {
        const inward = read?.receive.mbps ?? link.receive;
        const outward = read?.send.mbps ?? link.send;
        const rx = inward !== null ? Math.round(inward) : null;
        const tx = outward !== null ? Math.round(outward) : null;
        const generation = read?.receive.generation ?? null;
        return {
            in: rx,
            out: tx,
            // Each way said as the network's speed says it.
            side: rx === null ? "" : tx !== null && tx !== rx
                ? `${rx} Mb/s in \u00b7 ${tx} Mb/s out` : `${rx} Mb/s`,
            note: [generation ? `Wi-Fi ${generation}` : "", link.width ?? ""]
                .filter(part => part !== "").join(", ")
        };
    }

    // The signal a step in: at each antenna, on average, of the access
    // point's beacons, how far over the noise, and what the card sends at.
    function signalParts(name, card) {
        const link = linkOf(name, card);
        if (link === null)
            return [];
        const read = readOfCard(card);
        const key = id => `wifi:link:${card}:signal:${id}`;
        const dbm = value => `${value} dBm`;
        const rows = (read?.antennas ?? []).map((value, at) => fact(key(`antenna-${at}`),
            "antenna", dbm(value), { name: named("antenna", { n: at + 1 }), graph: level(value) }));
        const average = read?.average ?? link.average;
        if (average !== null)
            rows.push(fact(key("average"), "average", dbm(average), { graph: level(average) }));
        if (read && read.beacons !== null)
            rows.push(fact(key("beacons"), "beacons", dbm(read.beacons), {
                graph: level(read.beacons),
                note: "how loud the access point says it is, on average"
            }));
        if (link.margin !== null)
            rows.push(fact(key("margin"), "margin", `${link.margin} dB`, {
                note: link.noise !== null ? `the noise at ${link.noise} dBm` : ""
            }));
        if (read && read.power !== null)
            rows.push(fact(key("power"), "power", dbm(read.power), {
                note: "what this card sends at"
            }));
        return rows;
    }

    // The speed a step in: each way, the Wi-Fi it's spoken in, its coding,
    // width and channel, and what had to be sent again.
    function speedParts(name, card) {
        const link = linkOf(name, card);
        if (link === null)
            return [];
        const read = readOfCard(card);
        const key = id => `wifi:link:${card}:speed:${id}`;
        const rates = speeds(link, read);
        const rows = [];
        if (rates.in !== null)
            rows.push(fact(key("in"), "in", `${rates.in} Mb/s`));
        if (rates.out !== null)
            rows.push(fact(key("out"), "out", `${rates.out} Mb/s`));
        const coded = read ? read.receive : null;
        const generation = coded?.generation ?? null;
        if (generation !== null)
            rows.push(fact(key("generation"), "generation", `Wi-Fi ${generation}`, {
                note: generationSaid[generation] ?? ""
            }));
        const mcs = coded?.mcs ?? link.mcs;
        const streams = coded?.streams ?? link.streams;
        if (mcs !== null && mcs !== undefined)
            rows.push(fact(key("coding"), "coding", streams
                ? `MCS ${mcs}, ${streams} ${streams === 1 ? "stream" : "streams"}` : `MCS ${mcs}`, {
                note: "how densely it packs each signal; higher is faster"
            }));
        const width = coded?.width ? `${coded.width} MHz` : link.width ?? "";
        if (width !== "")
            rows.push(fact(key("width"), "width", width));
        const frequency = link.frequency ?? read?.frequency ?? null;
        const channel = read?.channel ?? null;
        if (frequency !== null || channel !== null)
            rows.push(fact(key("channel"), "channel", [channel !== null ? `${channel}` : "",
                frequency !== null ? bandOf(frequency) : ""].filter(part => part !== "")
                .join(", "), { note: frequency !== null ? `${number(frequency)} MHz` : "" }));
        if (read && read.retries !== null)
            rows.push(retriesRow(name, card, read));
        return rows;
    }

    // What each generation of Wi-Fi is called by its standard, for the
    // footer.
    readonly property var generationSaid: ({
        4: "802.11n, high throughput",
        5: "802.11ac, very high throughput",
        6: "802.11ax, high efficiency",
        7: "802.11be, extremely high throughput"
    })

    function bandOf(mhz) {
        return mhz >= 5925 ? ghz.ghz6 : mhz >= 5000 ? ghz.ghz5 : ghz.ghz2;
    }

    // What had to be sent again or never was, the beacons missed and what
    // was dropped coming in, since the card joined.
    function retriesRow(name, card, read) {
        const trouble = read.retries + (read.failed ?? 0);
        return holds(`wifi:link:${card}:speed:retries`, "retries", trouble === 0 ? "none"
            : `${number(read.retries)} sent again, ${number(read.failed ?? 0)} never sent`, {
                note: "since it connected"
            }, () => provider.retryParts(card));
    }

    function retryParts(card) {
        const read = readOfCard(card);
        if (read === null)
            return [];
        const key = id => `wifi:link:${card}:retries:${id}`;
        return [["retried", read.retries, "frames the access point didn't answer at first"],
            ["failed", read.failed, "frames given up on"],
            ["missed", read.missed, "the access point's beacons not heard"],
            ["dropped", read.dropped, "frames that came in and were thrown away"]]
            .filter(each => each[1] !== null)
            .map(each => fact(key(each[0]), each[0], number(each[1]), { note: each[2] }));
    }

    // How it's protected, said for what it means: who may join (anyone,
    // whoever has the password, or an account) and how hard it is to break;
    // a step in, the standard, how a card proves it may join, the
    // encryption, and whether the frames that join and leave are protected.
    function protectionOf(name) {
        const detail = detailOf(name);
        return detail ? detail.points[0].protection : null;
    }

    function guardRow(name) {
        const protection = protectionOf(name);
        if (!protection)
            return fact("wifi:protection", "protection", "none", {
                copy: "",
                note: "anyone near can read what it sends"
            });
        const account = protection.ways.some(way => /eap|8021x/i.test(way));
        const standard = protection.standard;
        const strength = /WPA3/.test(standard) ? "strong" : /WPA2/.test(standard) ? "good"
            : /WEP/.test(standard) ? "broken" : "weak";
        return holds("wifi:protection", "protection",
            `${account ? "an account" : "a password"}, ${strength}`, {
                keywords: standard,
                note: standard
            }, () => provider.guardParts(name));
    }

    readonly property var framesSaid: ({
        off: "unprotected",
        optional: "protected if the card can",
        required: "always protected"
    })

    function guardParts(name) {
        const protection = protectionOf(name);
        if (!protection)
            return [];
        const read = readOfCard(looked);
        const key = id => `wifi:protection:${id}`;
        const rows = [fact(key("standard"), "standard", protection.standard),
            fact(key("proof"), "proof", protection.ways.join(" or "), {
                note: "how a card proves it may connect"
            })];
        if (protection.ciphers.length > 0)
            rows.push(fact(key("cipher"), "cipher", protection.ciphers.join(" or "), {
                note: "what a card's own traffic is sent with"
            }));
        if (protection.group)
            rows.push(fact(key("group"), "group", protection.group, {
                note: "what's sent to every card at once is sent with"
            }));
        rows.push(fact(key("frames"), "frames", framesSaid[protection.management] ?? "", {
            note: read && read.protected !== null
                ? (read.protected ? "this card's are protected" : "this card's aren't")
                : "which stops anyone taking a card off it"
        }));
        return rows;
    }

    // How many access points are heard, and in which bands; stepped into,
    // each of them, the fastest first.
    function heardRow(name) {
        const detail = detailOf(name);
        const bands = detail.bands.map(count => `${count.points} on ${ghz[count.band]}`);
        return holds("wifi:heard", "heard",
            bands.join(" \u00b7 ").replace(/ GHz(?= \u00b7)/g, ""), {
                glyph: Theme.glyph.accessPoint
            }, () => {
                const now = provider.detailOf(name);
                return now ? provider.inOrder(now.points).map(point => provider.pointRow(point,
                    now)) : [];
            });
    }

    function inOrder(points) {
        const kept = order.filter(bssid => points.some(point => point.bssid === bssid));
        const fresh = points.filter(point => !kept.includes(point.bssid))
            .sort((a, b) => b.estimate - a.estimate || b.signal - a.signal);
        provider.order = kept.concat(fresh.map(point => point.bssid));
        return provider.order.map(bssid => points.find(point => point.bssid === bssid));
    }

    function fastestOf(point, detail) {
        return detail.points.length > 1 && detail.best !== null
            && detail.best.bssid === point.bssid;
    }

    // One access point, named by its band and channel: its signal, as a
    // bar too, and about how fast a card would get on with it at the side,
    // and whether it's the one joined or the fastest; in the footer, for
    // the fastest, why. A step in, its parts.
    function pointRow(point, detail) {
        const fastest = fastestOf(point, detail);
        const mark = point.joined ? " \u00b7 connected" : fastest ? " \u00b7 fastest" : "";
        return thingRow({
            key: `wifi:point:${point.bssid}`,
            name: named("point", { band: ghz[point.band], channel: point.channel }),
            side: `${point.signal} dBm \u00b7 ~${point.estimate} Mb/s${mark}`,
            graph: level(point.signal),
            glyph: strengthGlyph(point.bars, point.protection !== null),
            keywords: point.bssid,
            note: fastest ? fitting(detail.best.reasons) : ""
        }, () => provider.pointParts(detail.name, point.bssid));
    }

    // An access point's parts: its signal, about how fast a card would get
    // on with it, its width and Wi-Fi, how busy it is and how many are on
    // it, when it was last heard, why it's the fastest, and its hardware
    // address, which Return copies.
    function pointParts(name, bssid) {
        const detail = detailOf(name);
        const point = detail ? detail.points.find(each => each.bssid === bssid) ?? null : null;
        if (point === null)
            return [];
        const key = id => `wifi:point:${bssid}:${id}`;
        const rows = [fact(key("signal"), "signal", `${point.signal} dBm`, {
            graph: level(point.signal),
            note: `${point.bars} of 4 bars`
        }), fact(key("estimate"), "estimate", `about ${point.estimate} Mb/s`, {
            note: "how fast a card would get on with it"
        }), fact(key("width"), "width", `${point.width} MHz`), fact(key("generation"),
            "generation", point.generation ? `Wi-Fi ${point.generation}` : "older", {
                note: `${point.streams} ${point.streams === 1 ? "stream" : "streams"}`
            })];
        if (point.load) {
            rows.push(fact(key("busy"), "busy", `${point.load.busy}%`, {
                graph: { shape: "bar", values: [point.load.busy / 100], warn: false },
                note: "of the time, as it says"
            }));
            rows.push(fact(key("stations"), "stations", number(point.load.stations), {
                note: "cards it says are on it"
            }));
        }
        rows.push(fact(key("heard-ago"), "heard-ago",
            point.age < 10 ? "just now" : `${point.age} s ago`, { copy: "" }));
        if (fastestOf(point, detail) && detail.best.reasons.length > 0)
            rows.push(fact(key("fastest"), "fastest", detail.best.reasons.join(", ")));
        rows.push(fact(key("bssid"), "bssid", point.bssid));
        return rows;
    }

    // An act on a network, named by the vocabulary.
    function act(id, state, fields) {
        return Object.assign(fields, {
            key: `verb:${id}`,
            name: Words.name(`verb:${id}`, state, fields.fill),
            hue: "action"
        });
    }

    function verbs(network, locked) {
        const rows = [];
        // A sign-in is what's next while the network wants one.
        if (SignIn.network === network.name)
            rows.push(thing(act("wifi-sign-in", "", {
                verb: Words.verb("verb:wifi-sign-in"),
                glyph: Words.glyph("verb:wifi-sign-in"),
                run: () => SignIn.open()
            })));
        if (network.connected)
            rows.push(thing(act("wifi-leave", "", {
                glyph: Theme.glyph.offline,
                stays: true,
                run: () => Wifi.leave(network.name)
            })));
        else if (network.known || !locked.locked || locked.password)
            rows.push(thing(act("wifi-join", "", {
                verb: "connect",
                glyph: Theme.glyph.wireless,
                network: network.name,
                stays: true,
                state: () => provider.said(network, ""),
                run: () => provider.join(network)
            })));
        // Nix keeps the passwords and accounts of the networks it declares.
        // Each is made again under its act's name, so it's found by it.
        if (locked.password === true && !network.declared)
            rows.push(thing(Object.assign({}, asking(network, ""), act("wifi-password", "", {
                glyph: Theme.glyph.password,
                chip: "password"
            }))));
        if (locked.account === true && !network.declared)
            for (const method of ["peap", "ttls"])
                rows.push(thing(Object.assign({}, accountFor(network, method),
                    act("wifi-account", "", {
                        fill: { method: method.toUpperCase() },
                        glyph: Theme.glyph.password,
                        chip: "account"
                    }), { key: `verb:wifi-account-${method}` })));
        if (network.known && !network.declared) {
            rows.push(thing(act("wifi-trust", network.trusted ? "trusted" : "", {
                note: network.trusted ? "this machine stops finding others on it, and they it"
                    : "this machine finds others on it, and they it",
                glyph: Theme.glyph.trust,
                stays: true,
                run: () => Wifi.trust(network.name, !network.trusted)
            })));
            rows.push(thing(act("wifi-forget", "", {
                glyph: Theme.glyph.forget,
                stays: true,
                run: () => Wifi.forget(network.name)
            })));
        }
        return rows.concat(facts(network));
    }

    // How the join went: once the card is on the network, a password asked
    // for is done with, so the launcher steps back to the network, which
    // says it's joined, and stays (Joel, 2026-10-26: an act that changes
    // something "stays open and shows the new state on its row"); a
    // password that didn't work, or none given where one's needed, asks for
    // it; anything else says what happened.
    Connections {
        target: Wifi

        function onJoinEnded(name, stage, refused) {
            const launcher = provider.launcher;
            const at = launcher.selected ?? launcher.asking;
            const showing = launcher.open && at && at.network === name;
            if (refused === "") {
                // Joined, every ask it made is put away, its name, its
                // server's and its password, back to the network itself.
                if (showing && launcher.asking === at)
                    while (launcher.asking && launcher.asking.network === name
                        && launcher.held.length > 0)
                        launcher.back();
                return;
            }
            Report.say(`refused wifi ${name}`);
            const network = provider.networks.find(network => network.name === name)
                ?? provider.described(name, "personal", 0);
            provider.troubled = name;
            // One asking for an account is asked again from its account's
            // name, since a join that failed keeps nothing new.
            const account = provider.security(network).account === true;
            if (refused === "NeedsAccount" || (account && refused === "WrongPassword")) {
                provider.trouble = refused === "NeedsAccount" ? "it asks for an account"
                    : "that account's password didn't work";
                if (showing && !launcher.asking)
                    launcher.ask(provider.accountFor(network, "peap"));
                return;
            }
            if (refused === "WrongPassword" || refused === "NeedsPassword") {
                provider.trouble = refused === "NeedsPassword" ? "it asks for a password"
                    : network.known && !(at && at.secret) ? "its password has changed"
                    : "that password didn't work";
                if (showing && !launcher.asking)
                    launcher.ask(provider.asking(network, ""));
                return;
            }
            provider.trouble = {
                TimedOut: "it didn't answer in time",
                NotNearby: "it's out of reach",
                TurnedAway: "it didn't let this machine connect",
                ServerUnproven: "its server couldn't prove it's that domain's",
                Declared: "Nix keeps its password"
            }[refused] ?? "it couldn't be connected";
        }
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of everything()) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    // Wi-Fi holds its own off, after the networks so stepping in never
    // lands on it, as Bluetooth holds its radio (collection-order.md);
    // typed from the top, it's still found by name. With Wi-Fi off,
    // turning it on is all there is to do.
    function everything() {
        return on ? all.concat([radio]) : [radio];
    }

    function counted() {
        return !on ? "off" : `${all.length} in reach`;
    }
}
