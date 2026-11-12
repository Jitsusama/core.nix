// The Work Network, Joel's link to work through WARP, the VPN (Joel,
// 2026-11-11: "should we do the same for WARP and call it a work network"),
// answered in the field by its words ("warp", "vpn", "work" or "tunnel",
// as words.json lists them) and listed in Network after the Home Network,
// its sibling (VKCF3Z, 2026-11-11). The headline says how its link
// stands, "connected · 127 ms", in the warning colour while WARP finds the
// network degraded or its service doesn't answer; a step in are its
// switch, Disconnect WARP or Connect WARP as :disconnect and :connect name
// it, then its State and why, the network's Health as WARP judges it, its
// Settings (the mode, always on and the split tunnel first, each saying
// who set it), the tunnel's Round Trip and Loss as WARP estimates them,
// what's Waiting on the way as the network's shaper hears it, after the
// Round Trip (WarpWaiting.js),
// its Protocol, the Data Centre it ends in, its Endpoint and Last
// Handshake, its Traffic each way and its TLS, what its DNS proxy
// answered, and Through, WARP, named once (Warp.js works each out,
// WarpSeen.js the sparks and WarpSwitch.js the switch, in the words
// words.json gives warp). A machine whose system installs no warp-cli has
// no Work Network at all (home.nix enables this only where it does).
//
// What it shows comes from warp's daemon (modules/warp/src/serve.rs), over
// the socket it holds, so the launcher starts no process for WARP (Joel,
// 2026-11-10: "CLI execs is not performant"). Its one record, warp:link,
// is what `warp --json` prints, read only while it's watched: while it's
// shown, every two seconds, each read told as it lands; otherwise asked
// once as the answer is wanted, as a terminal's ask (`launcher show`)
// wants it. Whatever warp read last stands until the next, so the figures
// move in place rather than flickering back to waiting. What was read
// while it's shown is kept for the tunnel's sparks, whose drawers say
// once whose figures they are, and forgotten as the launcher opens or
// closes. The switch is the daemon's too, asked as an act, which leaves
// one the organisation holds as it is and says so. This is a step toward
// ready rows: Warp.js's working-out moves into the daemon once rows are
// served in the ready-rows form.
import qs
import Quickshell
import QtQuick
import "Warp.js" as Warp
import "WarpSeen.js" as WarpSeen
import "WarpSwitch.js" as WarpSwitch
import "WarpWaiting.js" as WarpWaiting
import "WarpDrawer.js" as WarpDrawer

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("warp")

    // Where the system has WARP; without it there's no WARP here.
    readonly property string warp: settings.warp ?? ""
    // How old what warp read may be and still be what's shown.
    readonly property int fresh: 2000
    // Whether a get or a switch is awaiting the daemon's answer, so a
    // second isn't asked over it.
    property bool looking: false
    property bool switching: false

    // What warp printed last, as JSON: the record, or { error, said }; null
    // before it first answers this opening.
    property var said: null
    property real saidAt: 0
    // What it has read of the tunnel since it was shown (WarpSeen.saw), for
    // the sparks, or null before a read; forgotten as the launcher closes.
    property var seen: null
    // What the last switch said, for its row's note, until the launcher
    // closes.
    property string switchSaid: ""
    // Whether warp was last asked for a terminal (`launcher show`) or the
    // catalogue while the launcher is closed. Their ask is answered at
    // once and can't wait on warp, so what it prints is kept for the next
    // ask, and the reads after spark as they do while shown.
    property bool askedOutside: false
    // How old what an ask from outside kept may be and still be said to
    // the next, rather than asked afresh.
    readonly property int keptOutside: 60000

    // Whether anyone can see what it says, so it's watched only then.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider || launcher.placed.includes("warp")
            || launcher.held.some(step => step.row.asker === provider))

    // warp's words, as words.json gives them.
    readonly property var words: Words.of("warp").said ?? {}

    function answer(typed) {
        if (warp === "" || asks(typed) === "")
            return null;
        const outside = launcher !== null && !launcher.open && launcher.cataloguing;
        if (outside && said !== null && Date.now() - saidAt > keptOutside) {
            said = null;
            seen = null;
        }
        if (launcher !== null && (launcher.open || launcher.cataloguing)
            && (said === null || Date.now() - saidAt > fresh)) {
            askedOutside = outside;
            look();
        }
        if (said === null)
            return waitingAnswer();
        return said.error !== undefined ? failedAnswer() : told();
    }

    // While warp first answers: held in its place, its figures to come,
    // saying how WARP stands where the margin's watch has heard it
    // (WarpState), so Network opens on "connected" rather than "\u2026".
    function waitingAnswer() {
        const standing = Warp.standing(WarpState.state, words);
        return {
            headline: standing?.headline ?? "\u2026",
            warns: standing?.warns ?? false,
            parted: true,
            rows: [],
            asking: true,
            glyph: Words.glyph("warp"),
            note: "",
            live: provider
        };
    }

    // warp couldn't read WARP: its service isn't answering, or said what
    // warp can't read, which the row says in its place; or there's no
    // warp-cli, so no WARP at all. Not answering, its drawer ends with
    // warp-cli's error as written, in the Name face, as something to copy
    // or look up (VKCF3Z, 2026-11-09). With a drawer it's a thing, as the
    // desk light's answer is, since the launcher draws only a thing's.
    function failedAnswer() {
        const failed = Warp.failed(said, words);
        if (failed === null)
            return null;
        return {
            thing: !!failed.error,
            headline: failed.headline,
            warns: failed.warns,
            whole: true,
            rows: [],
            glyph: Words.glyph("warp"),
            note: failed.note,
            drawer: failed.error ? {
                eyebrow: words.eyebrow,
                title: Words.name("warp"),
                facts: [{
                    label: Words.name("warp:error"),
                    value: failed.error,
                    identifies: true
                }]
            } : undefined,
            live: provider
        };
    }

    // How the link stands: its switch first, as :connect and :disconnect
    // name it, then every part. Its row in Network has a drawer of its own,
    // as every row has (VKCF3Z, 2026-11-12), which makes it a thing, since
    // the launcher draws only a thing's (WarpDrawer.js).
    function told() {
        const linked = Warp.told(said, settings, words);
        if (linked === null)
            return null;
        const parts = WarpSeen.drawn(
            WarpWaiting.among(linked.parts, said.waiting, settings, words), seen, settings, words);
        const drawn = WarpDrawer.drawer({
            parts,
            record: said,
            stood: WarpState.state,
            now: Date.now()
        }, { said: words, tells: Words.of("warp").tells ?? {} });
        return {
            headline: linked.headline,
            copy: linked.headline,
            warns: linked.warns,
            thing: true,
            drawer: {
                eyebrow: drawn.eyebrow,
                title: Words.name("warp"),
                body: drawn.body,
                facts: drawn.facts.map(fact => ({
                    label: Words.name(`warp:${fact.id}`),
                    value: fact.value,
                    identifies: fact.identifies === true
                }))
            },
            glyph: Words.glyph("warp"),
            rows: [switchRow()].concat(parts.map(partRow)),
            note: "",
            live: provider
        };
    }

    // A part's drawer, where it has one: its eyebrow saying whose figures
    // they are, once, its name, and each fact named by the vocabulary.
    function plate(part) {
        return {
            eyebrow: part.drawer.eyebrow,
            title: Words.name(`warp:${part.id}`),
            facts: part.drawer.facts.map(fact => ({
                label: Words.name(`warp:${fact.id}`),
                value: fact.value,
                figure: fact.figure ?? undefined
            }))
        };
    }

    // Disconnect WARP while it's up or on its way, Connect WARP while it's
    // down, as the vocabulary names each, so :disconnect and :connect name
    // them; drawn as `upAs` says it will be, where it's given. Its note
    // says who holds a switch the organisation holds, when WARP always on
    // comes back, or what the last switch said.
    function switchRow(upAs) {
        const up = upAs ?? WarpSwitch.up(said);
        const wordState = up ? "" : "off";
        const note = switchSaid !== "" ? switchSaid : WarpSwitch.switchNote(said, up, words);
        return thing(Object.assign({
            key: "warp:power",
            what: "",
            verb: Words.verb("warp:power", wordState),
            name: Words.name("warp:power", wordState),
            glyph: Words.glyph("warp:power", wordState),
            // Its words' state, so the : line names the command for it.
            wordState: wordState,
            hue: "action",
            note: note,
            // Held, it ends nothing: Return runs it and is refused with why.
            heldBy: WarpSwitch.holder(said),
            stays: true,
            state: () => provider.switchSaid !== "" ? provider.switchSaid : note,
            run: () => provider.turn(!up),
            undo: () => provider.turn(up)
        }, WarpSwitch.ends(said, up) ? Words.ending("warp:power") : {}));
    }

    // Has warp switch WARP, its row drawn as it will be until warp reads
    // it so, and put back with why if warp leaves it. warp itself refuses
    // a switch the organisation holds, asking warp-cli nothing.
    function turn(up) {
        if (switching)
            return;
        switching = true;
        switchSaid = "";
        becomes("warp:power", switchRow(up), Words.name("warp"), up ? "connect" : "disconnect");
        link.ask({ ask: "act", act: up ? "connect" : "disconnect" }, answer => {
            provider.switching = false;
            provider.switched(answer?.switched ?? {});
        });
    }

    // What warp said of a switch (modules/warp/src/switch.rs), as the
    // daemon answered with it: done, its note says when WARP comes back
    // where it will; held or refused, or not answered, the row goes back
    // and says why.
    function switched(printed) {
        const done = WarpSwitch.switched(printed, words);
        if (done.refused !== "") {
            refused("warp:power", done.refused);
            return;
        }
        switchSaid = done.note;
        look();
    }

    // A part as its row: a figure Return copies, or one holding parts of
    // its own, as the Settings, TLS and DNS do, which wears WARP's glyph
    // dimmed and steps in to them. An entry of a list, a split tunnel's
    // host or a DoH server's address, is named by itself.
    function partRow(part) {
        const entry = part.name !== undefined;
        const fields = {
            key: entry ? `warp:${part.id}:${part.name}` : `warp:${part.id}`,
            name: entry ? part.name : Words.name(`warp:${part.id}`),
            side: part.side,
            copy: part.copy,
            note: part.note,
            warns: part.warns,
            // A figure with a history sparks it, and its drawer says the
            // rest (WarpSeen.drawn).
            graph: part.graph,
            drawer: part.drawer ? () => provider.plate(part) : undefined,
            hue: "machine",
            glyph: ""
        };
        if (part.parts === undefined)
            return copyRow(fields);
        return holderRow(Object.assign(fields, { glyph: Words.glyph("warp") }),
            () => part.parts.map(partRow));
    }

    // Asks for the link once, where it isn't watched; watched, each read
    // is told as it lands.
    function look() {
        if (link.live || looking)
            return;
        looking = true;
        link.get(["warp:link"], 0, () => {
            provider.looking = false;
        });
    }

    // What warp read, kept until the next, unless the launcher closed
    // meanwhile, so the next opening asks afresh, or it was asked from
    // outside, so the next ask says it; and what the sparks keep of it.
    // A read already taken, as a get and a watch begun together both say
    // it, is taken once, so the sparks gain no point twice.
    function heard(changed) {
        const record = link.held["warp:link"];
        if (!changed.includes("warp:link") || record === undefined
            || record.read_at_ms === saidAt)
            return;
        const kept = askedOutside;
        askedOutside = false;
        if (launcher === null || !(launcher.open || launcher.cataloguing || kept))
            return;
        const printed = record.said ?? {};
        said = printed.status !== undefined || printed.error !== undefined
            ? printed : { error: "unreadable", said: words.printedNothing ?? "" };
        saidAt = record.read_at_ms;
        if (said.status !== undefined)
            seen = WarpSeen.saw(seen, said, saidAt);
        answered();
    }

    Connections {
        target: provider.launcher

        // Opened or closed, it asks afresh, so what a terminal was told
        // is never shown as though just asked.
        function onOpenChanged() {
            provider.said = null;
            provider.saidAt = 0;
            provider.seen = null;
            provider.switchSaid = "";
        }
    }

    // How WARP stands, heard before the link is read, said at once by the
    // row still waiting on it.
    Connections {
        target: WarpState

        function onStateChanged() {
            if (provider.said === null)
                provider.answered();
        }
    }

    // WARP's link, watched only while it's shown. A daemon that isn't
    // answering says nothing of it, so no old link is held as current.
    Daemon {
        id: link

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/warp.sock`
        kind: "warp"
        live: provider.shown
        keptThroughGap: false
        answersWhole: true
        onHeard: changed => provider.heard(changed)
    }
}
