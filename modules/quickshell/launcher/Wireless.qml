// Wi-Fi, a collection: the networks in the air, the one connected first,
// then those saved, then the rest by signal, each glyph saying how strong
// it is and whether it's locked, the footer how it's connected. Return
// disconnects the one connected or connects another, and the launcher stays,
// the foot saying how far it has got, until the network takes the card or
// says why not; a locked network met for the first time asks for its
// password in the field beside the padlock, and one asking for an account
// asks its name, its server's domain and its password in turn, the server
// proving it's that domain's before it's told. Stepping into a network
// gives, after what can be done to it, the network in detail, each a part
// named first: the card's Signal, its Speed, its Traffic each way since it
// connected, its Protection, its Access points and how long it's been
// Connected for, each holding more a step in. A network wanting a sign-in
// offers to sign in to it first. Turning Wi-Fi off and on is an action in
// the main search.
//
// wifid serves every row ready (modules/wifid/daemon/src/rows.rs and
// parts.rs): the radio's, the sign-in's and each network's with its acts,
// the words each asks for before it's sent among them, and, while a
// network is ahead (selected, stepped into or walked by a terminal), its
// drawer and every part inside it, worded in this account's vocabulary,
// which it's handed by its path, each in its place. They're held, placed
// and made into rows by ReadyRows.qml, its acts asked back on the same
// socket, so nothing here sorts, weighs, words, builds an act or reads a
// figure (Joel, 2026-11-10: "the daemons need to be the data layer, and
// they need to serve the launcher"; Y2YBTS's 17a4dc7e, 2026-11-13). What's
// left is what only the shell can do: open the sign-in page, turn the
// radio at :set and trust a network at :setl. The cards look for networks
// only while Wi-Fi's rows are shown, and read a link only while its network
// is ahead. What needs no row is in WirelessPlate.js.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Order.js" as Order
import "ReadyRows.js" as Ready
import "ActForms.js" as Forms
import "WirelessPlate.js" as Plate

Provider {
    id: provider

    what: "Wi-Fi"

    // A machine without wifid, or without a card, has no Wi-Fi to show.
    readonly property bool present: Wifi.described && Wifi.cards.length > 0
    readonly property bool on: present && Wifi.radio

    // :set wifi, nowifi and wifi! turn the radio as Wi-Fi Off and On do,
    // and :set wifi=Home-5G joins a network saved, as Return on it does;
    // one asking for words to be joined asks them, which only its row can.
    setOptions: !present ? ({}) : ({
        wifi: {
            kind: "switch",
            // Off drops the network joined, so :set nowifi is never run
            // by Return.
            ends: true,
            takes: true,
            now: () => !provider.on ? false : Wifi.joined?.network ?? true,
            // :set wifi= lists the networks saved, which it can join.
            choices: () => Wifi.saved.map(network => network.name),
            set: to => provider.setTo(to)
        }
    })

    function setTo(to) {
        // The radio is turned as asked whatever it seems to be, as :set sets
        // an option already set, so a word from wifid still on its way never
        // keeps :set nowifi from turning it off.
        if (typeof to === "boolean") {
            provider.turn(to);
            return "";
        }
        const asksPassword = ((Words.field.command ?? {}).asksPassword ?? "")
            .replace("{network}", to);
        // A network wifid serves is joined by its row's own act, so the foot
        // says how it goes, as Return on it does.
        const key = keyOf(to);
        const joining = actOf(ready.served(key), "verb:wifi-join");
        if (joining !== null) {
            if (Forms.fieldsOf(joining).length > 0)
                return asksPassword;
            ready.act(key, joining);
            return "";
        }
        if (!Wifi.isSaved(to))
            return asksPassword;
        if (!on)
            provider.turn(true);
        provider.actOn({ ask: "act", act: "join", network: to }, []);
        return "";
    }

    collection: !present ? null : Words.collection("wifi", {
        empty: on ? "Looking for networks\u2026" : "Wi-Fi is off."
    })

    // Looking for networks takes the card off its own channel for 5 to 6 s,
    // over each of its 100, so its link waits: a look every 10 s took the
    // gateway's ping from a p95 of 39 ms to 132 (2026-11-14). So the cards
    // look only while Wi-Fi's rows are there to show what they hear: Wi-Fi
    // or something in it held, or a find from the root showing a row of
    // its, the launcher open; or while a terminal's `launcher show` walked
    // into Wi-Fi (Launcher.qml's terminalSees), so a network asked for cold
    // is in the list. Until then the launcher open anywhere had them look.
    readonly property bool looking: {
        const launcher = provider.launcher;
        if (!on || launcher === null)
            return false;
        if (terminalLooks)
            return true;
        if (!launcher.open)
            return false;
        const held = launcher.held ?? [];
        if (held.length > 0)
            return held.some(step => step.row?.from === provider);
        return (launcher.typed ?? "") !== ""
            && (launcher.matches ?? []).some(row => row?.from === provider);
    }
    Timer {
        interval: 10000
        repeat: true
        triggeredOnStart: true
        running: provider.looking
        onTriggered: {
            if (provider.terminalLooks && !provider.launcher.open)
                provider.lookedFrom = Wifi.looks;
            provider.askWifid({ ask: "act", act: "scan" }, null);
        }
    }
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.from === provider)
    // How many looks wifid had finished when a terminal's look was asked
    // for, or -1 before it's asked, as the walk starts it: the look is done
    // once wifid says more.
    property int lookedFrom: -1
    onTerminalLooksChanged: {
        if (!terminalLooks)
            lookedFrom = -1;
    }

    // The rows wifid serves, held and placed as every place's ready rows
    // are, over one socket kept open, the networks ahead named so wifid
    // reads their links and serves their insides only then. A network's
    // row, its acts and each of its parts, is made once each change to it,
    // its acts before its parts.
    // Its rows as held, as Documents' are, which tests/wifi-acts.qml tells
    // records and answers through.
    readonly property alias served: ready
    ReadyRows {
        id: ready

        socket: "/run/wifid/rows"
        place: "wifi"
        watching: ({
                words: provider.settings.words ?? "",
                numbers: provider.settings.numbers ?? ""
            })
        ahead: provider.ahead
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
        insideAfterActs: true
        // Signing in opens wifid's page through niri (SignIn.qml), which
        // no act form says.
        actMade: (key, one) => one.key === "verb:wifi-sign-in" ? provider.signingIn() : null
    }

    // Asks wifid on its rows' own connection, answering null where it
    // isn't there to ask. Not `ask`, which is Provider's own, of a URL.
    function askWifid(said, answer) {
        if (ready.link)
            ready.link.ask(said, answer);
        else if (answer)
            answer(null);
    }

    // The key wifid serves a network's row under.
    function keyOf(name) {
        return `row:wifi:network:${name}`;
    }

    // The rows wifid serves at the top, in its order, as served.
    readonly property var rows: Ready.inside(ready.keys, ready.held, "")
        .map(key => ready.served(key)).filter(row => row !== null)

    // What a record is made into: a network's row, a part inside one, or,
    // the radio's and the sign-in's, the row as served, which radioRow and
    // signIn read.
    function madeFrom(row, key) {
        if (row.kind === "network")
            return made(row, key);
        if ((row.parent ?? "") !== "")
            return part(row, key);
        return row;
    }

    // A part inside a network, as wifid serves it: a figure copies what
    // it says, or what its act copies, and one holding more steps in, its
    // parts served beneath it. Return copies it, the right arrow steps in,
    // and Copy comes last, as every holder's does.
    function part(row, key) {
        const acts = row.acts ?? [];
        const copied = acts.find(one => typeof one.copy === "string");
        const fields = {
            key: row.thing ?? key,
            name: row.name ?? "",
            side: ready.sideOf(row),
            copy: copied ? copied.copy : "",
            note: ready.noteOf(row),
            hue: row.hue ?? "machine",
            glyph: Theme.glyph[row.glyph ?? ""] ?? "",
            graph: row.graph ?? undefined
        };
        return acts.some(one => one.step === "rows")
            ? holderRow(fields, () => ready.inside(key)) : copyRow(fields);
    }

    // A network's row as wifid serves it, its acts its verbs, Return doing
    // the first (ReadyRows.qml's thing), with what only the shell says of
    // it: that it's a network, for the network ahead; its trust, which
    // :setl sets as Vim's :setlocal sets a buffer's, where its row offers
    // Trust It; the drawer wifid serves on it while ahead, the one connected
    // only; and, connected, that its Return disconnects, which Return typed
    // in insert never does.
    function made(row, key) {
        const network = row.held ?? {};
        const connected = network.connected === true;
        return ready.thing(row, key, {
            thing: true,
            network: network.network ?? "",
            options: actOf(row, "verb:wifi-trust") ? {
                trust: {
                    kind: "switch",
                    now: () => network.trusted === true,
                    set: to => {
                        provider.trust(row, key, to);
                        return "";
                    }
                }
            } : undefined,
            drawer: connected && row.drawer ? () => ready.drawerOf(row) : undefined,
            ends: connected && network.signing !== true ? Words.ending("verb:wifi-leave").ends : ""
        });
    }

    // Trusted as said, by the act its row serves, which turns it the other
    // way; one already so is left as it is.
    function trust(row, key, trusted) {
        const act = actOf(row, "verb:wifi-trust");
        if (act !== null && act.ask.trusted === trusted)
            ready.act(key, act);
    }

    // Sign in to It, the act wifid serves a network wanting a sign-in first:
    // the page opens, so the launcher closes.
    function signingIn() {
        return thing({
            key: "verb:wifi-sign-in",
            wordsId: "verb:wifi-sign-in",
            what: "",
            verb: Words.verb("verb:wifi-sign-in"),
            name: Words.name("verb:wifi-sign-in"),
            glyph: Words.glyph("verb:wifi-sign-in"),
            hue: "action",
            stays: false,
            run: () => SignIn.open()
        });
    }

    // A network as its row's held fields say it, for Wi-Fi's own drawer.
    function networkOf(row) {
        return Object.assign({}, row.held, { name: row.held.network, served: row });
    }

    // The row wifid serves of a kind, the radio's or the sign-in's.
    function servedOf(kind) {
        return rows.find(row => row.kind === kind) ?? null;
    }

    readonly property var radio: radioRow(on)

    // Wi-Fi Off while the radio's on, as it is, or as it will be once
    // turned, both as wifid words them: the row it serves, or the row its
    // act says it becomes. The vocabulary words it only until wifid has
    // served a row at all.
    function radioRow(lit) {
        const row = servedOf("radio");
        const said = !row ? null : row.held.on === lit ? row
            : actOf(row, "wifi:radio")?.becomes ?? null;
        return thing({
            key: "wifi:radio",
            // An act, not a network, so it says no kind (docs/design.md).
            what: "",
            verb: Words.verb("wifi:radio", lit ? "" : "off"),
            name: said ? said.name : Words.name("wifi:radio", lit ? "" : "off"),
            glyph: said ? Theme.glyph[said.glyph] : Words.glyph("wifi:radio", lit ? "" : "off"),
            // Its words' state, so the : line names the command for it.
            wordState: lit ? "" : "off",
            hue: "machine",
            stays: true,
            run: () => provider.turn(!lit)
        });
    }

    // A row's act as wifid serves it, by its key: what to ask, and the row
    // it becomes once done. Null where wifid says the row has no such act.
    function actOf(row, key) {
        return (row?.acts ?? []).find(act => act.key === key) ?? null;
    }

    // Asks an act of wifid, on its rows' own connection, and puts each row
    // it held as it will be back if wifid won't, or isn't there, saying why
    // in wifid's words; one holding no row journals why.
    function actOn(ask, keys) {
        provider.askWifid(ask, answer => {
            const why = Ready.refusal(answer, Words.field.becoming ?? {});
            if (why !== "" && keys.length === 0)
                console.info(`launcher: wifi ${ask.act} refused: ${
                    answer?.error?.refused ?? "no answer"}`);
            for (const key of keys)
                provider.refused(key, why);
        });
    }

    // The radio is asked to turn as said, whatever its row says, drawn at
    // once as wifid says it becomes, until wifid serves it so, and put back
    // if it won't (launcher/Becoming.js).
    function turn(lit) {
        becomes("wifi:radio", radioRow(lit), provider.what, lit ? "on" : "off");
        actOn({ ask: "act", act: "radio", on: lit }, ["wifi:radio"]);
    }

    things: !present ? [] : signIn ? [signIn, radio] : [radio]

    // While a network wants a sign-in, it's what's next.
    readonly property var signIn: {
        const row = servedOf("sign-in");
        return !row ? null : thing({
            key: "wifi:sign-in",
            what: "",
            verb: Words.verb("wifi:sign-in"),
            name: row.name,
            glyph: Theme.glyph[row.glyph],
            hue: row.hue,
            run: () => SignIn.open()
        });
    }

    // Every network in the air, and the one joined even when it hides its
    // name, in wifid's order, each with what its row is served with.
    readonly property var networks: !on ? [] : rows.filter(row => row.kind === "network")
        .map(row => networkOf(row))

    // The side as wifid serves it, for Wi-Fi's own drawer.
    function sideOf(network) {
        return (network.served?.side ?? []).map(item => item.said).join(" \u00b7 ");
    }

    // Each network's row as made, in wifid's order; a change to a part or
    // a row beside the networks leaves the list as it was, so it's told
    // changed only once a network's row is new.
    readonly property var all: !on ? [] : Ready.inside(ready.keys, ready.held, "")
        .filter(key => ready.served(key)?.kind === "network").map(key => ready.rowAt(key))
    property var allTold: []

    // What a network's spots are made of, so a closing makes again only
    // the parts of a network that changed (Launcher.qml's heldSpots): its
    // served row with its strength left out, and each part inside it by
    // its key, name and acts, never a figure, since the signal moves every
    // scan and the parts' spots don't show it. Joel's closing at 14:36 on
    // 2026-11-11 made Wi-Fi's 17 rows again. Null for a row not made of a
    // network now, so it's made again. Wireless's figures served ready as
    // parts, apart from their values, would end it.
    function spotsStampOf(thing) {
        if (ready.stampOf(thing) === null)
            return null;
        const key = ready.madeOf[thing.key];
        const row = ready.served(key);
        const steady = Object.assign({}, row, { glyph: null, side: null,
            held: Object.assign({}, row.held, { strength: null }) });
        const parts = Ready.inside(ready.keys, ready.held, key).map(one => {
            const part = ready.served(one);
            return [one, part?.name ?? "", (part?.acts ?? []).map(act => act.key ?? "")];
        });
        return JSON.stringify([steady, parts]);
    }
    onAllChanged: {
        if (all.length === allTold.length && all.every((row, at) => row === allTold[at]))
            return;
        allTold = all;
        changed();
    }

    // The network stepped into, for as long as it, or its parts, are
    // shown: in the open launcher, or walked through by a terminal's
    // `launcher show` (Launcher.qml's terminalSees), so its Signal, Speed
    // and Carried are read for a terminal with the launcher closed; or the
    // network connected while its row is selected, for its drawer. What
    // the launcher held when it closed asks nothing.
    readonly property string looking: {
        const launcher = provider.launcher;
        if (!launcher)
            return "";
        const network = row => row?.key === `wifi:${row?.network}`;
        const step = launcher.open ? launcher.held.find(step => network(step.row)) : undefined;
        const walked = step?.row ?? (launcher.terminalSees?.walked ?? []).find(network);
        if (walked)
            return walked.network;
        // A row elsewhere drawing that row's drawer as its own, as Now's
        // Wi-Fi does (Now.qml's borrows), looks at it too.
        const selected = launcher.open ? launcher.selected : null;
        // Wi-Fi's own row reads the network joined, for placePlate.
        if (selected?.key === `collection:${settings.name}` && Wifi.joined)
            return Wifi.joined.network;
        const borrowed = selected?.borrows?.place === settings.name
            ? all.find(row => row.key === selected.borrows.key) : undefined;
        const chosen = network(selected) ? selected : borrowed;
        return chosen && Wifi.isJoined(chosen.network) ? chosen.network : "";
    }

    // The network whose row is selected, joined or not, so its parts are
    // ready by the time it's stepped into.
    readonly property string selected: {
        const at = provider.launcher?.open ? provider.launcher.selected : null;
        return at?.key === `wifi:${at?.network}` ? at.network : "";
    }

    // The networks wifid is told are ahead, so it reads their links and
    // serves their insides (ready-rows.md's Notes): the one looked at and
    // the one selected, once each.
    readonly property var ahead: [looking, selected]
        .filter((name, at, names) => name !== "" && names.indexOf(name) === at)
        .map(name => keyOf(name))

    // Whether the network looked at waits on wifid's parts of it, so what
    // it says is still "not read yet" (Provider.stillReading): until wifid
    // has served a part inside it. A terminal walked into Wi-Fi with no
    // network heard waits too, for the look its walk asked for, until
    // wifid says a look is done, so with nothing in range it waits for one
    // look and no longer, and with wifid not answering, not at all. A list
    // already heard is said at once, and the look freshens it for the next
    // ask. With the launcher open, its own looks keep the list, and a
    // terminal asks for none.
    stillReading: looking !== "" && ready.inside(keyOf(looking)).length === 0
        || terminalLooks && !(launcher?.open ?? false) && on && Wifi.described
            && Wifi.nearby.length === 0 && (lookedFrom === -1 || lookedFrom === Wifi.looks)

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
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
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

    // Wi-Fi's own row's drawer (WirelessPlate.js), led by the network
    // joined, whose drawer wifid serves once the row's selected (looking).
    function placePlate() {
        const joined = networks.find(network => network.connected);
        return Plate.plate({ on: on, networks: networks, joined: joined ? {
            name: joined.name, side: sideOf(joined),
            drawn: joined.served ? ready.drawerOf(joined.served) : null } : null }, {
            name: Words.name("wifi"), said: Words.of("wifi").said ?? {},
            leading: [Words.name("wifi:signal"), Words.name("wifi:connected")] });
    }
}
