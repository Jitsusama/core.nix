// Wi-Fi, a collection: the networks in reach, the one joined first, then
// those saved, then the rest by signal, each glyph saying how strong it is
// and whether it's locked, the footer giving the signal and how it's
// secured. Return leaves the one joined or joins another, and the launcher
// stays, saying it's joining, until the network takes it or says why not;
// a locked network met for the first time, or one whose password has
// changed, asks for its password in the field beside the padlock. Turning
// Wi-Fi off and on is an action in the main search. Everything comes from
// NetworkManager over D-Bus through Quickshell, so no process is asked, and
// the radio scans only while the launcher is open.
import qs
import Quickshell
import Quickshell.Networking
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    // The radio NetworkManager looks after, the one joined to a network if
    // there are several: one it leaves alone, such as an access point's,
    // can't join anything.
    readonly property var device: {
        const radios = Networking.devices.values
            .filter(device => device.type === DeviceType.Wifi && device.nmManaged);
        return radios.find(device => device.connected) ?? radios[0] ?? null;
    }
    readonly property bool on: device !== null && Networking.wifiEnabled

    collection: device === null ? null : {
        noun: "networks",
        title: "Wi-Fi",
        glyph: glyph(0xf05a9),
        ask: "which network?",
        empty: "Listening for networks\u2026"
    }

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    // Looking for networks costs the radio's power, so it looks only while
    // the launcher is there to show them.
    Binding {
        target: provider.device
        property: "scannerEnabled"
        value: provider.launcher !== null && provider.launcher.open
        when: provider.device !== null
    }

    readonly property var radio: thing({
        key: "wifi:radio",
        verb: on ? "turn off" : "turn on",
        name: on ? "Turn Wi-Fi Off" : "Turn Wi-Fi On",
        keywords: "wifi wireless radio network airplane",
        glyph: glyph(on ? 0xf05aa : 0xf05a9),
        hue: "machine",
        run: () => Networking.wifiEnabled = !provider.on
    })

    things: device === null ? [] : [radio]

    // How a network is locked, in the words its settings use, and whether
    // a password typed here opens it: one needing a certificate or a WEP
    // key is set up in nmtui instead.
    function security(network) {
        switch (network.security) {
        case WifiSecurityType.Open:
            return { words: "open", locked: false };
        case WifiSecurityType.Owe:
            return { words: "open, encrypted", locked: false };
        case WifiSecurityType.WpaPsk:
            return { words: "WPA", locked: true, password: true };
        case WifiSecurityType.Wpa2Psk:
            return { words: "WPA2", locked: true, password: true };
        case WifiSecurityType.Sae:
            return { words: "WPA3", locked: true, password: true };
        case WifiSecurityType.WpaEap:
            return { words: "WPA Enterprise", locked: true };
        case WifiSecurityType.Wpa2Eap:
            return { words: "WPA2 Enterprise", locked: true };
        case WifiSecurityType.Wpa3SuiteB192:
            return { words: "WPA3 Enterprise", locked: true };
        case WifiSecurityType.Leap:
            return { words: "LEAP", locked: true };
        case WifiSecurityType.StaticWep:
        case WifiSecurityType.DynamicWep:
            return { words: "WEP", locked: true };
        default:
            return { words: "secured somehow", locked: true };
        }
    }

    // Signal in four steps, from the icon font's own: with and without a
    // lock, and an outline for none at all.
    function bars(strength) {
        return Math.min(4, Math.ceil(strength * 4));
    }

    function strengthGlyph(strength, locked) {
        const codes = locked ? [0xf092c, 0xf0921, 0xf0924, 0xf0927, 0xf092a]
            : [0xf092f, 0xf091f, 0xf0922, 0xf0925, 0xf0928];
        return glyph(codes[bars(strength)]);
    }

    // The one joined, then those saved, then by signal, a step at a time so
    // a scan's small changes don't reorder them, then by name.
    readonly property var networks: {
        if (!on)
            return [];
        const rank = network => network.connected ? 0 : network.known ? 1 : 2;
        return device.networks.values.filter(network => network.name !== "").sort((a, b) =>
            rank(a) - rank(b) || bars(b.signalStrength) - bars(a.signalStrength)
            || a.name.localeCompare(b.name));
    }

    // The network being joined, and what it said if it wouldn't be.
    property var joining: null
    property string trouble: ""

    function join(network) {
        trouble = "";
        joining = network;
        network.connect();
    }

    function joinWith(network, password) {
        trouble = "";
        joining = network;
        network.connectWithPsk(password);
    }

    // What the footer says while a network is joined, or wouldn't be.
    function said(network, otherwise) {
        if (joining !== network)
            return otherwise;
        return trouble !== "" ? trouble : `joining ${network.name}\u2026`;
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
            network: network,
            state: () => provider.said(network, why),
            given: password => provider.joinWith(network, password),
            note: locked.words
        });
    }

    function made(network) {
        const locked = security(network);
        const joined = network.connected;
        // Met for the first time behind a password, it asks for one.
        const asks = !joined && !network.known && locked.password === true;
        const elsewhere = !joined && !network.known && locked.locked && !locked.password;
        const note = `${Math.round(network.signalStrength * 100)}% signal · ${locked.words}`;
        return member({
            key: `wifi:${network.name}`,
            verb: joined ? "leave" : elsewhere ? "set up" : "join",
            name: network.name,
            glyph: strengthGlyph(network.signalStrength, locked.locked),
            hue: "machine",
            side: joined ? "connected" : network.known ? "saved" : "",
            note: elsewhere ? `${note} · set up in nmtui` : note,
            network: network,
            state: () => provider.said(network, note),
            // Joining keeps the launcher, which says how it went.
            stays: !joined && !elsewhere,
            later: elsewhere,
            secret: asks,
            ask: asks ? "password" : "",
            chip: network.name,
            given: asks ? password => provider.joinWith(network, password) : null,
            run: joined ? () => network.disconnect()
                : elsewhere ? () => Niri.spawn(["kitty", "nmtui", "connect"])
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
                run: () => network.disconnect()
            }));
        else
            rows.push(thing({
                key: "verb:wifi-join",
                verb: "join",
                name: "Join it",
                glyph: glyph(0xf05a9),
                hue: "action",
                network: network,
                stays: true,
                state: () => provider.said(network, ""),
                run: () => provider.join(network)
            }));
        if (locked.password === true)
            rows.push(Object.assign(asking(network, ""), {
                key: "verb:wifi-password",
                name: "Join it with a new password",
                glyph: glyph(0xf0306),
                hue: "action",
                chip: "password"
            }));
        if (network.known)
            rows.push(thing({
                key: "verb:wifi-forget",
                name: "Forget it",
                glyph: glyph(0xf16c5),
                hue: "action",
                run: () => network.forget()
            }));
        return rows;
    }

    // How the network being joined went: once joined, the launcher's done,
    // if it's still on that network; a password refused, or a saved one
    // that's changed, asks for it; anything else says what happened.
    Connections {
        target: provider.joining

        function onConnectedChanged() {
            const network = provider.joining;
            if (!network || !network.connected)
                return;
            provider.joining = null;
            const launcher = provider.launcher;
            const at = launcher.selected ?? launcher.asking;
            if (launcher.open && at && at.network === network)
                launcher.open = false;
        }

        function onConnectionFailed(reason) {
            const network = provider.joining;
            const launcher = provider.launcher;
            Report.say(`refused wifi ${network ? network.name : ""}`);
            const at = launcher.selected ?? launcher.asking;
            if (reason === ConnectionFailReason.NoSecrets && provider.security(network).password) {
                provider.trouble = network.known && !(at && at.secret)
                    ? "its password has changed" : "that password didn't work";
                if (launcher.open && at && at.network === network && !launcher.asking)
                    launcher.ask(provider.asking(network, ""));
                return;
            }
            provider.trouble = reason === ConnectionFailReason.WifiAuthTimeout
                ? "it didn't answer in time"
                : reason === ConnectionFailReason.WifiNetworkLost ? "it's out of reach"
                : "it didn't let this machine join";
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
