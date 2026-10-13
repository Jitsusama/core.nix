// Wi-Fi, a collection: the networks in the air, the one joined first, then
// those saved, then the rest by signal, each glyph saying how strong it is
// and whether it's locked, the footer how it's joined. Return leaves the one
// joined or joins another, and the launcher stays, saying how far the join
// has got, until the network takes the card or says why not; a locked
// network met for the first time, or one whose password didn't work, asks
// for its password in the field beside the padlock. Turning Wi-Fi off and
// on is an action in the main search. Everything comes from wifid over its
// socket (Wifi.qml), and the cards look for networks only while the
// launcher is open.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // A machine without wifid, or without a card, has no Wi-Fi to show.
    readonly property bool present: Wifi.described && Wifi.cards.length > 0
    readonly property bool on: present && Wifi.radio

    collection: !present ? null : {
        noun: "networks",
        title: "Wi-Fi",
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
        keywords: "wifi wireless radio network airplane",
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
        keywords: "wifi portal captive login hotel",
        glyph: glyph(0xf05a9),
        hue: "machine",
        run: () => SignIn.open()
    })

    function home() {
        return SignIn.network !== "" ? [signIn] : [];
    }

    // How a network is joined, in words, and whether a password typed here
    // opens it: one asking for an account can't be joined from here yet.
    function security(network) {
        switch (network.security) {
        case "open":
            return { words: "open", locked: false };
        case "personal":
            return { words: "a password", locked: true, password: true };
        default:
            return { words: "an account", locked: true };
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

    function join(network, password) {
        troubled = "";
        trouble = "";
        Wifi.join(network.name, password);
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
        // Met for the first time behind a password, it asks for one.
        const asks = !joined && !network.known && locked.password === true;
        const elsewhere = !joined && !network.known && locked.locked && !locked.password;
        const signal = network.strength > 0 ? `${network.strength} of 4 bars · ` : "";
        const note = `${signal}${locked.words}`;
        return member({
            key: `wifi:${network.name}`,
            verb: joined ? "leave" : elsewhere ? "" : "join",
            name: network.name,
            glyph: strengthGlyph(network.strength, locked.locked),
            hue: "machine",
            side: joined ? "connected" : network.known ? "saved" : "",
            note: elsewhere ? `${note} · not joinable from here yet` : note,
            network: network.name,
            state: () => provider.said(network, note),
            // Joining keeps the launcher, which says how it went.
            stays: !joined,
            secret: asks,
            ask: asks ? "password" : "",
            chip: network.name,
            given: asks ? password => provider.join(network, password) : null,
            run: joined ? () => Wifi.leave(network.name)
                : elsewhere ? () => {}
                : () => provider.join(network),
            verbs: () => provider.verbs(network, locked)
        });
    }

    readonly property var all: networks.map(network => made(network))
    onAllChanged: changed()

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
        // Nix keeps the passwords of the networks it declares.
        if (locked.password === true && !network.declared)
            rows.push(Object.assign(asking(network, ""), {
                key: "verb:wifi-password",
                name: "Join it with a new password",
                glyph: glyph(0xf0306),
                hue: "action",
                chip: "password"
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
        return rows;
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
