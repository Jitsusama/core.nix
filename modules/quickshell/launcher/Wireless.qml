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
// a part named first: the card's Signal, its Speed, its Traffic each way
// since it connected, its Protection, its Access points and how long it's
// been Connected for, each holding more a step in. A network wanting a
// sign-in offers to sign in to it first. Turning Wi-Fi off and on is an
// action in the main search.
//
// wifid serves every row ready (modules/wifid/daemon/src/rows.rs and
// parts.rs): the radio's, the sign-in's and each network's, and, while a
// network is ahead (selected, stepped into or walked by a terminal), its
// drawer and every part inside it, worded in this account's vocabulary,
// which it's handed by its path, each in its place. They're held and
// placed by ReadyRows.qml, so nothing here sorts, weighs, words or reads
// a figure (Joel, 2026-11-10: "the daemons need to be the data layer, and
// they need to serve the launcher"). What's done to a network, joins
// among them, asks wifid on the same socket, and only acts its row lists.
// The cards look for networks only while the launcher is open, and read a
// link only while its network is ahead.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Order.js" as Order
import "Placed.js" as Placed
import "ReadyRows.js" as Ready

Provider {
    id: provider

    what: "Wi-Fi"

    // A machine without wifid, or without a card, has no Wi-Fi to show.
    readonly property bool present: Wifi.described && Wifi.cards.length > 0
    readonly property bool on: present && Wifi.radio

    // :set wifi, nowifi and wifi! turn the radio as Wi-Fi Off and On do,
    // and :set wifi=Home-5G joins a network saved, as Return on it does;
    // one never joined asks its password, which only its row can ask.
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
        if (!Wifi.isSaved(to))
            return ((Words.field.command ?? {}).asksPassword ?? "").replace("{network}", to);
        if (!on)
            provider.turn(true);
        provider.join({ name: to }, null, null);
        return "";
    }

    collection: !present ? null : Words.collection("wifi", {
        empty: on ? "Looking for networks\u2026" : "Wi-Fi is off."
    })

    // Looking for networks costs the radio's power, so the cards look only
    // while the launcher is there to show what they hear, or while a
    // terminal's `launcher show` walked into Wi-Fi (Launcher.qml's
    // terminalSees), so a network asked for cold is in the list.
    Timer {
        interval: 10000
        repeat: true
        triggeredOnStart: true
        running: provider.on && provider.launcher !== null
            && (provider.launcher.open || provider.terminalLooks)
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
    // row, and each of its parts, is made here once each change to it.
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
        onHeard: (changed, gone) => {
            for (const key of gone)
                delete provider.verbsOf[key];
        }
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
            return made(networkOf(row));
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

    // A network as its row's held fields say it, for its acts.
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
    // it held as it will be back if wifid won't, or isn't there.
    function actOn(ask, keys) {
        provider.askWifid(ask, answer => {
            const refused = answer === null ? "Failed"
                : answer.error ? answer.error.refused ?? "Failed" : "";
            for (const key of keys)
                provider.refused(key, why(refused));
        });
    }

    // What wifid is asked, each row it changes drawn at once as wifid says
    // it becomes, until wifid serves it so, and put back if it won't
    // (launcher/Becoming.js).
    // The radio is asked to turn as said, whatever its row says.
    function turn(lit) {
        becomes("wifi:radio", radioRow(lit), provider.what, lit ? "on" : "off");
        actOn({ ask: "act", act: "radio", on: lit }, ["wifi:radio"]);
    }

    // A network's acts are only those its row lists: wifid offers each
    // only where it can be done, so one it doesn't list is never asked.
    function leave(network) {
        const act = actOf(network.served, "verb:wifi-leave");
        if (act === null)
            return;
        const key = `wifi:${network.name}`;
        becomes(key, made(networkOf(act.becomes)), network.name, "disconnect");
        // Inside the network, Disconnect It goes, as it will once it's left.
        becomes("verb:wifi-leave", null, network.name, "disconnect");
        actOn(act.ask, [key, "verb:wifi-leave"]);
    }

    // Trusted as said, as :setl sets it; wifid's row says how it reads
    // turned, and one already so reads as it is.
    function trust(network, trusted) {
        const act = actOf(network.served, "verb:wifi-trust");
        if (act === null)
            return;
        const turned = act.ask.trusted === trusted ? networkOf(act.becomes) : network;
        becomes("verb:wifi-trust", trustRow(turned), network.name,
            trusted ? "trust" : "distrust");
        actOn(Object.assign({}, act.ask, { trusted: trusted }), ["verb:wifi-trust"]);
    }

    // Forgotten, the network is one never met, and what only a network saved
    // can have done to it goes.
    function forget(network) {
        const act = actOf(network.served, "verb:wifi-forget");
        if (act === null)
            return;
        const key = `wifi:${network.name}`;
        becomes(key, made(networkOf(act.becomes)), network.name, "forget");
        becomes("verb:wifi-trust", null, network.name, "forget");
        becomes("verb:wifi-forget", null, network.name, "forget");
        actOn(act.ask, [key, "verb:wifi-trust", "verb:wifi-forget"]);
    }

    // Why wifid wouldn't, in the foot's words, or "" where it did.
    function why(refused) {
        if (refused === "")
            return "";
        return {
            NotJoined: "not connected",
            NotSaved: "not saved",
            Declared: "nix keeps it"
        }[refused] ?? "wifid couldn't";
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

    // What's typed here to open a network: a password, or an account with
    // its server and password, and how wifid words it.
    function security(network) {
        return {
            words: network.served?.note ?? "",
            locked: network.security !== "open",
            password: network.security !== "open",
            account: network.security === "enterprise"
        };
    }

    // Signal in four bars, as the theme's marks draw it: with and without a
    // lock, and an outline for none heard, for a network wifid hasn't
    // served.
    function strengthGlyph(bars, locked) {
        const kind = locked ? "lockedSignal" : "signal";
        return Theme.glyph[`${kind}${Math.max(0, Math.min(4, bars))}`];
    }

    // Every network in the air, and the one joined even when it hides its
    // name, in wifid's order, each with what its row is served with.
    readonly property var networks: !on ? [] : rows.filter(row => row.kind === "network")
        .map(row => networkOf(row))

    // The side as wifid serves it, and as it says an act leaves it. A
    // network it hasn't served, as one a join refused may be, says none.
    function sideOf(network) {
        return (network.served?.side ?? []).map(item => item.said).join(" \u00b7 ");
    }

    // A network wifid hasn't served, as one a join refused may be, with
    // what its acts need to know of it.
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

    // The join under way, as { name, stage }, and how many have been
    // asked, so the steps of one given way to a newer are let go.
    property var joining: null
    property int joins: 0

    // Joins a network on wifid's rows socket, with its password when it
    // asks one and the account ({ method, identity, serverDomain }) where
    // it asks for one. wifid answers each step until the card is done,
    // and a join already under way gives way to a new one, as wifid gives
    // it way.
    function join(network, password, account) {
        troubled = "";
        trouble = "";
        const name = network.name;
        const said = { ask: "act", act: "join", network: name };
        if (password !== undefined && password !== null)
            said.password = password;
        if (account !== undefined && account !== null)
            said.account = account;
        joins += 1;
        const mine = joins;
        joining = { name: name, stage: "joining" };
        provider.askWifid(said, answer => {
            if (mine !== joins)
                return;
            if (answer !== null && !answer.error && answer.done === false) {
                joining = { name: name, stage: answer.stage };
                return;
            }
            joining = null;
            // Not there to ask, or wifid gone mid-join, it couldn't.
            if (answer === null || answer.error)
                joinEnded(name, "", answer?.error?.refused ?? "Failed");
            else
                joinEnded(name, answer.stage, "");
        });
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
        if (joining && joining.name === network.name)
            return `${stages[joining.stage] ?? stages.joining} ${network.name}\u2026`;
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
        // How it stands, then its bars, are the side's; what guards it
        // follows its name (VKCF3Z, 2026-11-09: "connected · 4 of 4 bars"
        // at the right of "Lantern  a password").
        const note = locked.words;
        const opening = met && locked.account ? provider.accountFor(network, "peap") : null;
        // Joined but wanting a sign-in, Return signs in, as it's all that's
        // left to do there; otherwise it leaves a network joined or joins one.
        const signing = network.signing === true;
        return member({
            thing: true,
            key: `wifi:${network.name}`,
            verb: signing ? Words.verb("verb:wifi-sign-in") : joined ? "disconnect" : "connect",
            name: network.served?.name ?? network.name,
            glyph: network.served ? Theme.glyph[network.served.glyph]
                : strengthGlyph(network.strength, locked.locked),
            hue: network.served?.hue ?? "machine",
            // Connected, as Return connects it and its access point says it
            // is; join is a call's word (docs/glossary.md).
            side: sideOf(network),
            note: note,
            network: network.name,
            // Its own option, which :setl sets as Vim's :setlocal sets a
            // buffer's: whether it's trusted, as its Trust It row turns it,
            // on a network saved here and not declared.
            options: actOf(network.served, "verb:wifi-trust") ? {
                trust: {
                    kind: "switch",
                    now: () => network.trusted,
                    set: to => {
                        provider.trust(network, to);
                        return "";
                    }
                }
            } : undefined,
            state: () => provider.said(network, note),
            // The one connected shows the drawer wifid serves on its row
            // while it's ahead, as it is once selected.
            drawer: network.connected && network.served
                ? () => ready.drawerOf(network.served) : null,
            // Joining or leaving keeps the launcher, which says how it went;
            // signing in opens the page, so it closes.
            stays: !signing,
            secret: asks,
            ask: asks ? "password" : opening ? opening.ask : "",
            chip: network.name,
            given: asks ? password => provider.join(network, password)
                : opening ? opening.given : null,
            run: signing ? () => SignIn.open() : joined ? () => provider.leave(network)
                : () => provider.join(network),
            verbs: () => provider.verbsKept(network, locked),
            // Joined, its Return disconnects, which Return typed in insert
            // never does.
            ends: joined && !signing ? Words.ending("verb:wifi-leave").ends : ""
        });
    }

    // Each network's row as made, in wifid's order; a change to a part or
    // a row beside the networks leaves the list as it was, so it's told
    // changed only once a network's row is new.
    readonly property var all: !on ? [] : Ready.inside(ready.keys, ready.held, "")
        .filter(key => ready.served(key)?.kind === "network").map(key => ready.rowAt(key))
    property var allTold: []
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

    // An act on a network, named by the vocabulary.
    function act(id, state, fields) {
        return Object.assign(fields, Words.ending(`verb:${id}`), {
            key: `verb:${id}`,
            name: Words.name(`verb:${id}`, state, fields.fill),
            hue: "action",
            // Its words' state, so the : line names the command for it.
            wordState: state
        });
    }

    // Trust It, as the network is trusted now, or will be.
    function trustRow(network) {
        return thing(act("wifi-trust", network.trusted ? "trusted" : "", {
            note: network.trusted ? "this machine stops finding others on it, and they it"
                : "this machine finds others on it, and they it",
            glyph: Theme.glyph.trust,
            stays: true,
            run: () => provider.trust(network, !network.trusted)
        }));
    }

    // Each network's acts as last made, by its row's key, with what they
    // were made from. A freshening asks every network's acts again a
    // minute after a closing (Launcher.qml's spotsFreshened), and made
    // Wi-Fi's 14 to 20 again each time with nothing changed (2026-11-10),
    // so they're made again only once the network's row or its sign-in is
    // new. Its parts follow them, each made once a change to it
    // (ReadyRows.qml), the first set apart from the acts above it.
    property var verbsOf: ({})
    function verbsKept(network, locked) {
        const key = network.served?.key;
        if (!key)
            return verbs(network, locked);
        const acts = Placed.madeOnce(verbsOf, key, [network.served, SignIn.network],
            () => verbs(network, locked));
        const parts = ready.inside(key);
        if (parts.length > 0)
            parts[0].gap = true;
        return acts.concat(parts);
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
        // A network left was one joined, so its password is known and
        // joining it again is how u puts it back.
        if (actOf(network.served, "verb:wifi-leave"))
            rows.push(thing(act("wifi-leave", "", {
                glyph: Theme.glyph.offline,
                stays: true,
                run: () => provider.leave(network),
                undo: () => provider.join(network)
            })));
        else if (!network.connected && (network.known || !locked.locked || locked.password))
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
        if (actOf(network.served, "verb:wifi-trust"))
            rows.push(trustRow(network));
        if (actOf(network.served, "verb:wifi-forget"))
            rows.push(thing(act("wifi-forget", "", {
                glyph: Theme.glyph.forget,
                stays: true,
                run: () => provider.forget(network)
            })));
        return rows;
    }

    // How the join went: once the card is on the network, a password asked
    // for is done with, so the launcher steps back to the network, which
    // says it's joined, and stays (Joel, 2026-10-26: an act that changes
    // something "stays open and shows the new state on its row"); a
    // password that didn't work, or none given where one's needed, asks for
    // it; anything else says what happened. It's given the stage the join
    // reached, or why it couldn't, as wifid names it.
    function joinEnded(name, stage, refused) {
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
        const network = networks.find(network => network.name === name)
            ?? described(name, "personal", 0);
        troubled = name;
        // One asking for an account is asked again from its account's
        // name, since a join that failed keeps nothing new.
        const account = security(network).account === true;
        if (refused === "NeedsAccount" || (account && refused === "WrongPassword")) {
            trouble = refused === "NeedsAccount" ? "it asks for an account"
                : "that account's password didn't work";
            if (showing && !launcher.asking)
                launcher.ask(accountFor(network, "peap"));
            return;
        }
        if (refused === "WrongPassword" || refused === "NeedsPassword") {
            trouble = refused === "NeedsPassword" ? "it asks for a password"
                : network.known && !(at && at.secret) ? "its password has changed"
                : "that password didn't work";
            if (showing && !launcher.asking)
                launcher.ask(asking(network, ""));
            return;
        }
        trouble = {
            TimedOut: "it didn't answer in time",
            NotNearby: "it's out of reach",
            TurnedAway: "it didn't let this machine connect",
            ServerUnproven: "its server couldn't prove it's that domain's",
            Declared: "Nix keeps its password"
        }[refused] ?? "it couldn't be connected";
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
}
