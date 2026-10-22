// Wi-Fi, a collection: the networks in the air, the one joined first, then
// those saved, then the rest by signal, each glyph saying how strong it is
// and whether it's locked, the footer how it's joined. Return leaves the one
// joined or joins another, and the launcher stays, saying how far the join
// has got, until the network takes the card or says why not; a locked
// network met for the first time, or one whose password didn't work, asks
// for its password in the field beside the padlock, and one asking for an
// account asks its name, its server's domain and its password in turn, the
// server proving it's that domain's before it's told. Stepping into a
// network gives, after what can be done to it, the network in detail: the
// card's link, how it's protected, and how many access points are heard,
// which steps into each of them, with the one wifid would pick and why. Turning Wi-Fi off and
// on is an action in the main search. Everything comes from wifid over its
// socket (Wifi.qml), and the cards look for networks only while the
// launcher is open.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "Wi-Fi"

    // A machine without wifid, or without a card, has no Wi-Fi to show.
    readonly property bool present: Wifi.described && Wifi.cards.length > 0
    readonly property bool on: present && Wifi.radio

    collection: !present ? null : {
        noun: "networks",
        title: "Wi-Fi",
        words: ["wifi", "wireless", "internet", "network", "ssid", "hotspot"],
        glyph: glyph(0xf05a9),
        ask: "which network?",
        empty: on ? "Listening for networks\u2026" : "Wi-Fi is off"
    }

    function glyph(code) {
        return String.fromCodePoint(code);
    }

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
        verb: on ? "turn off" : "turn on",
        name: on ? "Turn Wi-Fi Off" : "Turn Wi-Fi On",
        keywords: "wifi wlan wireless radio network internet airplane mode flight",
        glyph: glyph(on ? 0xf05aa : 0xf05a9),
        hue: "machine",
        run: () => Wifi.turn(!provider.on)
    })

    things: !present ? [] : SignIn.network !== "" ? [signIn, radio] : [radio]

    // While a network wants a sign-in, it's what's next.
    readonly property var signIn: thing({
        key: "wifi:sign-in",
        verb: "sign in",
        name: `Sign In to ${SignIn.network}`,
        keywords: "wifi portal captive login log in hotel airport cafe",
        glyph: glyph(0xf05a9),
        hue: "machine",
        run: () => SignIn.open()
    })

    function home() {
        return SignIn.network !== "" ? [signIn] : [];
    }

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

    // Signal in four bars, from the icon font's own: with and without a
    // lock, and an outline for none heard.
    function strengthGlyph(bars, locked) {
        const codes = locked ? [0xf092c, 0xf0921, 0xf0924, 0xf0927, 0xf092a]
            : [0xf092f, 0xf091f, 0xf0922, 0xf0925, 0xf0928];
        return glyph(codes[Math.max(0, Math.min(4, bars))]);
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
            verb: "join",
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
        joining: "joining",
        gettingAddress: "getting an address from",
        checking: "seeing how far it reaches:"
    })

    // What the footer says while a network is joined, or wouldn't be.
    function said(network, otherwise) {
        if (Wifi.joining && Wifi.joining.name === network.name)
            return `${stages[Wifi.joining.stage] ?? "joining"} ${network.name}\u2026`;
        if (troubled === network.name && trouble !== "")
            return trouble;
        return otherwise;
    }

    // The field asking for a network's password, the padlock beside it.
    function asking(network, why) {
        const locked = security(network);
        return thing({
            key: `wifi:${network.name}:password`,
            verb: "join",
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
        return member({
            key: `wifi:${network.name}`,
            verb: joined ? "leave" : "join",
            name: network.name,
            glyph: strengthGlyph(network.strength, locked.locked),
            hue: "machine",
            side: joined ? "connected" : network.known ? "saved" : "",
            note: note,
            network: network.name,
            state: () => provider.said(network, note),
            // Joining keeps the launcher, which says how it went.
            stays: !joined,
            secret: asks,
            ask: asks ? "password" : opening ? opening.ask : "",
            chip: network.name,
            given: asks ? password => provider.join(network, password)
                : opening ? opening.given : null,
            run: joined ? () => Wifi.leave(network.name) : () => provider.join(network),
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
        Wifi.watch(looking);
    }

    // Its rows change as wifid hears more of it.
    Connections {
        target: Wifi

        function onDetailChanged() {
            if (provider.looking !== "")
                provider.changed();
        }
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

    // A network in detail, after what can be done to it: the link of each
    // card on it, how it's protected, and how many access points are heard
    // in which bands, which steps into each of them. Return copies a row,
    // and its footer says more of it.
    function facts(network) {
        const detail = Wifi.detail;
        if (!detail || detail.name !== network.name || detail.points.length === 0)
            return [];
        const rows = detail.joined.map(link => linkRow(link));
        rows.push(guardRow(detail.points[0].protection));
        rows.push(heardRow(detail));
        rows[0].gap = true;
        return rows;
    }

    // A card's link, as its driver last read it.
    function linkRow(link) {
        const rates = link.receive === null ? ""
            : link.send !== null && link.send !== link.receive
                ? `${link.receive} Mb/s in, ${link.send} out` : `${link.receive} Mb/s`;
        const read = [link.signal !== null ? `${link.signal} dBm` : "", rates]
            .filter(part => part !== "").join(" \u00b7 ");
        const name = read !== "" ? read : "nothing read yet";
        return copyRow({
            key: `wifi:link:${link.card}`,
            name: name,
            side: link.card,
            glyph: glyph(0xf05a9),
            copy: `${link.card}: ${name}`,
            note: fitting([
                link.mcs === null ? ""
                    : `MCS ${link.mcs}` + (link.streams ? ` \u00d7${link.streams}` : ""),
                link.width ?? "",
                link.margin !== null ? `${link.margin} dB over the noise`
                    : link.average !== null ? `${link.average} dBm on average` : ""
            ])
        });
    }

    // How it's protected: the standard, whether its management frames are
    // too, and in the footer how a card proves it may join and the cipher.
    function guardRow(protection) {
        if (!protection)
            return copyRow({
                key: "wifi:protection",
                name: "open",
                side: "nothing it sends is encrypted",
                glyph: glyph(0xf0fc6)
            });
        const frames = {
            off: "frames unprotected",
            optional: "frames protected if the card can",
            required: "frames always protected"
        }[protection.management];
        const proving = `${protection.ways.join(" or ")} \u00b7 ${protection.ciphers.join("/")}`;
        return copyRow({
            key: "wifi:protection",
            name: protection.standard,
            side: frames,
            glyph: glyph(0xf033e),
            copy: `${protection.standard}: ${proving}, ${frames}`,
            note: proving
        });
    }

    // How many access points are heard, and in which bands; stepped into,
    // each of them, the fastest first.
    function heardRow(detail) {
        const heard = detail.points.length;
        const bands = detail.bands.map(count => `${count.points} on ${ghz[count.band]}`);
        const side = bands.join(" \u00b7 ").replace(/ GHz(?= \u00b7)/g, "");
        return thing({
            key: "wifi:heard",
            name: heard === 1 ? "1 access point" : `${heard} access points`,
            side: side,
            glyph: glyph(0xf0003),
            hue: "voice",
            verb: "see all",
            targets: () => inOrder(detail.points).map(point => pointRow(point, detail))
        });
    }

    function inOrder(points) {
        const kept = order.filter(bssid => points.some(point => point.bssid === bssid));
        const fresh = points.filter(point => !kept.includes(point.bssid))
            .sort((a, b) => b.estimate - a.estimate || b.signal - a.signal);
        provider.order = kept.concat(fresh.map(point => point.bssid));
        return provider.order.map(bssid => points.find(point => point.bssid === bssid));
    }

    // One access point: its band and channel, width and signal lined up,
    // about how fast a card would get on with it at the side, whether it's
    // the one joined or the fastest, and in the footer its address, its
    // Wi-Fi, how busy it is, or for the fastest, why. Return copies its
    // address.
    function pointRow(point, detail) {
        const fastest = detail.points.length > 1 && detail.best !== null
            && detail.best.bssid === point.bssid;
        const mark = point.joined ? "joined \u00b7 " : fastest ? "fastest \u00b7 " : "";
        const generation = point.generation ? `Wi-Fi ${point.generation}` : "older";
        const load = point.load;
        return copyRow({
            key: `wifi:point:${point.bssid}`,
            // Lined up by no-break spaces, which a row drawn styled keeps.
            name: `${ghz[point.band]} ch ${point.channel}`.padEnd(13, "\u00a0")
                + `${point.width} MHz`.padStart(7, "\u00a0")
                + `${point.signal} dBm`.padStart(9, "\u00a0"),
            side: `${mark}~${point.estimate} Mb/s`,
            glyph: strengthGlyph(point.bars, point.protection !== null),
            copy: point.bssid,
            keywords: `${point.bssid} access point`,
            note: fastest ? fitting(detail.best.reasons) : fitting([
                point.bssid,
                `${generation} \u00d7${point.streams}`,
                point.age >= 10 ? `heard ${point.age} s ago` : "",
                load ? `${load.busy}% busy` : "",
                load ? `${load.stations} on it` : ""
            ])
        });
    }

    function verbs(network, locked) {
        const rows = [];
        if (network.connected)
            rows.push(thing({
                key: "verb:wifi-leave",
                name: "Leave it",
                glyph: glyph(0xf05aa),
                hue: "action",
                run: () => Wifi.leave(network.name)
            }));
        else if (network.known || !locked.locked || locked.password)
            rows.push(thing({
                key: "verb:wifi-join",
                verb: "join",
                name: "Join it",
                glyph: glyph(0xf05a9),
                hue: "action",
                network: network.name,
                stays: true,
                state: () => provider.said(network, ""),
                run: () => provider.join(network)
            }));
        // Nix keeps the passwords and accounts of the networks it declares.
        if (locked.password === true && !network.declared)
            rows.push(Object.assign(asking(network, ""), {
                key: "verb:wifi-password",
                name: "Join it with a new password",
                glyph: glyph(0xf0306),
                hue: "action",
                chip: "password"
            }));
        if (locked.account === true && !network.declared)
            for (const method of ["peap", "ttls"])
                rows.push(Object.assign(accountFor(network, method), {
                    key: `verb:wifi-account-${method}`,
                    name: `Join it with an account over ${method.toUpperCase()}`,
                    glyph: glyph(0xf0306),
                    hue: "action",
                    chip: "account"
                }));
        if (network.known && !network.declared) {
            rows.push(thing({
                key: "verb:wifi-trust",
                name: network.trusted ? "Stop trusting it" : "Trust it",
                note: network.trusted ? "this machine stops finding others on it, and they it"
                    : "this machine finds others on it, and they it",
                glyph: glyph(0xf0483),
                hue: "action",
                run: () => Wifi.trust(network.name, !network.trusted)
            }));
            rows.push(thing({
                key: "verb:wifi-forget",
                name: "Forget it",
                glyph: glyph(0xf16c5),
                hue: "action",
                run: () => Wifi.forget(network.name)
            }));
        }
        return rows.concat(facts(network));
    }

    // How the join went: once the card is on the network, the launcher's
    // done, if it's still showing it; a password that didn't work, or none
    // given where one's needed, asks for it; anything else says what
    // happened.
    Connections {
        target: Wifi

        function onJoinEnded(name, stage, refused) {
            const launcher = provider.launcher;
            const at = launcher.selected ?? launcher.asking;
            const showing = launcher.open && at && at.network === name;
            if (refused === "") {
                if (showing)
                    launcher.open = false;
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
                TurnedAway: "it didn't let this machine join",
                ServerUnproven: "its server couldn't prove it's that domain's",
                Declared: "Nix keeps its password"
            }[refused] ?? "it couldn't be joined";
        }
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
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

    // With Wi-Fi off, turning it on is all there is to do.
    function everything() {
        return on ? all.slice() : [radio];
    }

    function counted() {
        return !on ? "off" : `${all.length} in reach`;
    }
}
