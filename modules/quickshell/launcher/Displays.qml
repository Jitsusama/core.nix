// Displays, a collection: the displays niri drives, the built-in one first,
// each with its size, refresh rate and scale in the footer. Return moves
// focus to one, or turns on one that's off; the right arrow lists its
// Scale, Refresh, Resolution and Rotation as parts saying how each is set,
// each stepping into its choices (which what's typed there finds too, so
// "scale 1.5" needs no step in), and turning it off while another is on.
// "colour depth", "refresh rate" and the like ask the focused display's
// part, or each display's while several are on. niri forgets these
// choices when its configuration is next read, so they last the session,
// and what lasts belongs in the configuration. Each screen's row comes
// ready from the windows daemon (modules/windows, screens.rs), its name,
// how it stands, what it runs and its drawer, light and all, said there,
// with what the : line's :setl takes for each setting (screens/line.rs)
// and its connector, but no record of niri's; the daemon asks
// niri for its screens as a display comes or goes, so the shell asks
// niri nothing of them itself. A change made here, from a row or the :
// line, is the daemon's to ask of niri, which says nothing of it in its
// stream, so the daemon looks again once niri answers. What a display's
// EDID says of itself, who made it and when, its size, native mode,
// refresh range, colour depth, colours, white point, primaries and HDR,
// desk-facts writes to a file (modules/desk-facts), which the daemon
// follows and serves, in its drawer and as rows inside the screen's
// after its settings and the ways it turns off, named first as parts are
// (Colour Depth "10 bit"), Return copying one (screens/facts.rs), so the
// shell reads no file for them. How many screens there are and how many
// are on, beside the place's name, is the daemon's summary.
//
// The panel's backlight is the one brightness, lid shut or not: the
// backlight daemon (modules/backlight) sets every monitor it reaches to
// the same share, so one Brightness says it, in one screen's place: the
// built-in one while it's on, else the first lit monitor that follows it,
// as desk-facts says by a brightness among its facts (lid shut and
// docked, the monitor). The windows daemon serves it leading that
// screen's settings (modules/windows, light/brightness.rs): its share as
// a bar, its levels a step in, which set it as the brightness keys do, so
// the backlight daemon, which follows the room while the lid is open,
// learns it as Joel's pick; then, on the built-in screen, the Room light
// its sensor hears. "brightness" asks it, and "brightness
// 40" sets it on Return. The backlight and the room's light are the
// machine daemon's (modules/machine, light.rs), which the windows daemon
// reads while a screen is watched ahead (modules/windows, light.rs), so
// the shell reads neither the lights nor a file for them: which screen
// holds the Brightness is its served row's, and the share :set
// brightness and "brightness 40" say, and what they say while it's read,
// the place's line serves (screens/line.rs's place).
//
// The backlight daemon warms every screen at night from sunset to
// sunrise, and toward the room's light where its sensor sees colour, and
// says how that stands, which the machine daemon serves as it changes
// (modules/backlight, modules/machine's steered.rs). The windows daemon
// serves the Warmth as Screens' own, beside the screens, in the same
// place with the lid open or shut (modules/windows, light/warmth.rs;
// VKCF3Z, 2026-11-15): its side the warmth now, its note what holds it,
// and Return stepping into the Night Light, then Follow the Room while a
// screen in the room is lit, switches the backlight daemon is told over
// the session's bus and keeps, then the room's white, the night's sunset
// and sunrise and its white, each time said by the shell's clock.
// "screen warmth" asks it, and the : line's :set nightlight and :set
// warmth ask what its switches ask. The place's : line, :set screens,
// brightness, nightlight and warmth, is the daemon's too, served on its
// summary (screens/line.rs's place), so the shell only offers what it
// serves (placeOptions).
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../Hearing.js" as Hearing
import "Order.js" as Order
import "Seen.js" as Seen

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // The screens' rows as the windows daemon serves them, each holding
    // its connector, how it stands and its drawer; Turn Every Screen Off is
    // the daemon's row too, while one is lit. The screen whose drawer
    // shows is watched ahead, and the one holding the Brightness while the
    // lights can be seen, as by words asking the Room Light, or a terminal
    // looks, so the daemon reads its light from the machine daemon only
    // then (modules/windows, light.rs), and once as the shell starts, so
    // :set brightness says the share before Screens is first opened.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        place: "displays"
        beside: ["summary"]
        ahead: [provider.drawerWanted,
            provider.lightsSeen || provider.terminalLooks || !provider.lightReadOnce
                ? provider.backlitKey : ""]
            .filter((key, at, keys) => key !== "" && keys.indexOf(key) === at)
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    // The place's : line as the windows daemon serves it on its summary
    // (screens/line.rs's place): :set brightness=60 sets the screen as its
    // Brightness levels do, while it's served; :set nightlight and
    // nonightlight turn the night light on and off, and :set nowarmth
    // gives the screens their own white by day, as the Warmth's switches
    // do, while each is served; :set noscreens turns every screen off,
    // doing Turn Every Screen Off's act. Each says how it stands as served.
    readonly property var placeLine: {
        void ready.placings;
        return ready.held["summary:displays"]?.line ?? {};
    }
    setOptions: placeOptions(placeLine)

    // The : line's options for `served`, the place's line, each asking the
    // way the daemon serves for it (placeWay).
    function placeOptions(served) {
        const line = name => provider.placeLine[name] ?? null;
        const options = {};
        for (const [name, one] of Object.entries(served)) {
            const set = (to, change, later) => provider.placeWay(name, to, later);
            options[name] = one.kind === "number"
                ? { kind: "number", least: one.least, most: one.most,
                    now: () => line(name)?.now ?? 0, set: set }
                : { kind: "switch", now: () => line(name)?.now === true, set: set };
        }
        return options;
    }

    // What :set's option `name` turned `to` asks, as the daemon serves it:
    // a number its ask at the share typed; a switch's way, the ask its
    // row's Return makes, or the act of a row by key, done as Return does
    // it, so Turn Every Screen Off closes the launcher first. A way not
    // served asks nothing, as turning a switch the way it stands.
    function placeWay(name, to, later) {
        const served = placeLine[name] ?? null;
        if (served?.kind === "number") {
            brighten(to, later);
            return "";
        }
        const way = served?.ways?.[to ? "on" : "off"] ?? null;
        if (way?.ask) {
            lineAsked(way.ask, later, name);
        } else if (way?.act) {
            const act = (ready.served(way.row)?.acts ?? []).find(one => one.key === way.act);
            if (act)
                ready.act(way.row, act);
        }
        return "";
    }

    // The Brightness as the place's line serves it: the share read, or
    // null while it's read, so no old level is said as now, and what its
    // row says beside it, the share, nothing in its first second, then
    // that it isn't read yet; null while none is served.
    readonly property var servedBrightness: {
        const served = placeLine.brightness ?? null;
        return served === null ? null : {
            share: typeof served.now === "number" ? served.now : null,
            said: served.said ?? ""
        };
    }
    // Whether a share has been read since the shell started, until when
    // the screen holding the Brightness is watched ahead, as the shell's
    // first look, so its levels are served for :set brightness and
    // "brightness 40" before Screens is first opened.
    property bool lightReadOnce: false
    onServedBrightnessChanged: {
        if (servedBrightness?.share !== null && servedBrightness?.share !== undefined)
            lightReadOnce = true;
    }

    // A change made to a screen here says nothing in niri's stream, nor
    // does a key waking the screens, so the windows daemon is asked to look
    // at niri's screens again, here and as the launcher opens, and tells
    // the rows that moved; the asking is the daemon's, off this thread.
    function refresh() {
        ready.link?.ask({ ask: "look" }, null);
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.refresh();
            // Shut, no drawer shows, so none is watched ahead.
            else
                provider.drawerWanted = "";
        }
    }

    // Sets the screen to a share, for :set brightness= and "brightness
    // 40", asking the windows daemon what its Brightness levels ask, at
    // that share, as the place's line serves it, so it goes as a level's
    // Return does; the daemon keeps it lit at least a little. Nothing is
    // asked while no Brightness is served.
    function brighten(share, later) {
        const ask = placeLine.brightness?.ask ?? null;
        if (ask !== null)
            lineAsked(Object.assign({}, ask, { share: share }), later, `brightness ${share}`);
    }

    // Asks the windows daemon what a served row's Return asks, for the
    // : line or typed words, the Brightness saying the light and a switch
    // how it stands once the daemon tells them. No row asked it, so none is
    // drawn becoming: a refusal, in the daemon's words or the launcher's for
    // an answer that never came, goes to `later` as the : line's other
    // answers do (lineAsk), else to the journal, naming `what` was asked.
    function lineAsked(ask, later, what) {
        if (!ready.link)
            return;
        const becoming = Words.field.becoming ?? {};
        ready.link.ask(ask, answer => {
            const why = !answer ? becoming.noAnswer ?? ""
                : answer.error ? answer.error.said || (becoming.couldNot ?? "") : "";
            if (later)
                later(why, true);
            else if (why !== "")
                console.info(`launcher: ${what} refused: ${why}`);
        });
    }

    // Setting the screen to a share, Return doing it, its note the share
    // now as the served Brightness says it, or what that says while it's
    // read: nothing for its first second, then that it isn't read yet.
    function setTo(share) {
        const brightness = servedBrightness;
        if (brightness === null)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: partSaid("", "setTo").replace("{share}", kept),
            verb: Words.command("command:brightness").verb,
            stays: true,
            run: () => provider.brighten(kept),
            rows: [],
            note: brightness.share !== null
                ? partSaid("", "now").replace("{share}", brightness.share) : brightness.said
        };
    }

    // Whether a light's answer shows, out at the top, which the launcher
    // asks again as the light moves, so what follows the room moves as it
    // does.
    readonly property bool lightShown: launcher !== null && launcher.open
        && (launcher.held ?? []).length === 0 && launcher.answer?.live === provider

    // What the light's rows inside the screen holding it say, the
    // Brightness and the Room Light, which moves as the light does, so a
    // light's answer showing is asked again as it moves. Asked as the turn
    // ends, never at once: the rows move as lightsSeen changes, inside
    // its own binding, and an answer asked there makes the rows
    // lightsSeen reads again, a binding loop (the answers VM test,
    // 2026-11-10).
    readonly property string lightSaid: {
        void ready.placings;
        const output = ready.served(backlitKey)?.held?.output ?? "";
        const parts = [`displays:${output}:brightness`, `displays:${output}:room`];
        return JSON.stringify(ready.inside(backlitKey)
            .filter(row => parts.includes(row.key))
            .map(row => [row.side, row.note]));
    }
    onLightSaidChanged: Qt.callLater(Hearing.putOff(provider.lightTold))

    // Timed from the line that told the light (Hearing.js).
    readonly property var lightTold: Hearing.carried(() => provider.lightAnswered())
    function lightAnswered() {
        if (lightShown)
            answered();
    }

    // Whether a terminal's `launcher show` walked into Screens or to a
    // screen (Launcher.qml's terminalSees), so its drawer's light is read
    // with the launcher closed too.
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.from === provider || String(row?.key ?? "").startsWith("display:"))
    // A terminal's look waits, as the launcher shows the light once it
    // opens (Provider.stillReading), for the windows daemon to say the
    // Brightness in the drawer of the screen holding it, read or not yet
    // read, which it does within a second of its look (modules/windows,
    // light.rs's WAITED); a machine with no backlight waits for nothing.
    stillReading: terminalLooks && !(launcher?.open ?? false) && ready.state !== "not answering"
        && servedBrightness !== null && brightnessServed(backlitKey) === ""

    // The row's key of the screen whose place holds the Brightness, read
    // or not, as the windows daemon says on its row (screens.rs's
    // backlit), or "". Only one holds it, since every screen shows the
    // same level.
    readonly property string backlitKey: {
        void ready.placings;
        return ready.keys.find(key => ready.served(key)?.held?.lights === true) ?? "";
    }

    // What the drawer the daemon serves on the row of `key` says of the
    // Brightness, found by the part it says it is (light.rs's facts), ""
    // while it says none.
    function brightnessServed(key) {
        void ready.placings;
        const fact = (ready.served(key)?.drawer?.facts ?? [])
            .find(each => each.part === "brightness");
        return fact?.said ?? "";
    }

    // Whether the lights can be seen, so are watched: Screens' answer,
    // Screens or a screen stepped into or found, a place holding Screens,
    // words typed asking the Brightness or the Room light, or a terminal's
    // look (Seen.js), as Heat is read; not every page the launcher opens on.
    readonly property bool lightsSeen: Seen.seen(launcher, provider, ["displays"],
        typed => brightnessTyped.test(typed.trim().toLowerCase())
            || ["brightness", "room"].includes(partsAsked[asks(typed)]))
        || terminalLooks

    // Its name, as the windows daemon serves it on its row (screens.rs's
    // named), else its connector's.
    function named(output) {
        return ready.served(screenKey(output))?.name ?? output.name;
    }

    // The screens niri lays out, lit or dark, each by its connector's
    // name, in the order the daemon serves them: the built-in first, as
    // it's always there, then by connector. One unplugged, laid out on
    // nothing, its row says stands off.
    readonly property var lit: {
        void ready.placings;
        return ready.keys.map(key => ready.served(key))
            .filter(row => row?.kind === "screen" && row.held?.standing !== "off")
            .map(row => ({ name: row.held.output }));
    }

    // What a served row is made into: a screen's, by the connector it
    // holds, Turn Every Screen Off, or what's inside a screen's row: a lit
    // one's settings, their choices and its Variable Refresh
    // (modules/windows, screens/settings.rs), the ways it turns off or on
    // (acts.rs) and what its EDID says (facts.rs).
    function madeFrom(row, key) {
        if (key === "row:displays:sleep")
            return sleepRow(row);
        // A way and a fact are made as served: a way's Return asks the
        // daemon, the last lit screen's closing the launcher, and a fact's
        // copies what it says.
        if (row.kind === "action" || row.kind === "fact")
            return ready.thing(row, key);
        // A setting with choices offers them, so what's typed from the
        // screen finds them, as Sound's served settings do; one with
        // nothing to choose only copies its value. The Warmth offers its
        // switches and figures the same way (light/warmth.rs).
        if (row.kind === "setting" || row.kind === "warmth")
            return ready.thing(row, key,
                { offers: (row.acts ?? []).some(one => one.step === "rows") });
        // A choice keeps its setting's name as a hidden word, so "scale
        // 1.5" finds 1.5\u00d7 from outside, and fills :setl's command as its
        // Return would type it; it's a choice, so it has no verbs of its own.
        // Its words are made ready to match here, as Provider.thing makes a
        // row's, since the launcher reads hidden words only made ready.
        if (row.kind === "choice")
            return ready.thing(row, key, {
                keywords: Fuzzy.kept(`${row.name ?? ""} ${row.held?.keywords ?? ""}`.trim()),
                fill: row.held?.fill,
                verbs: null,
                targets: undefined
            });
        // A switch keeps its name as it flips, its verb saying which way
        // in its act's words: Variable Refresh's, or the Warmth's Night
        // Light and Follow the Room.
        if (row.kind === "switch") {
            const on = row.held?.on === true;
            return ready.thing(row, key, {
                wordState: on ? "" : "off",
                verb: Words.verb(row.acts?.[0]?.key ?? "displays:variable", on ? "" : "off"),
                verbs: null,
                targets: undefined
            });
        }
        return row.kind === "screen" && row.held?.output
            ? made({ name: row.held.output }, row, key) : null;
    }

    // A screen's row as the daemon says it, its name, how it stands and
    // what it runs, and its Return, focusing it or turning it on, the
    // daemon's act (modules/windows, screens/acts.rs). Its verbs are its
    // light, made here, and what the daemon serves inside it, so Return,
    // done, closes the launcher as it did, and the key language reads none
    // of its act.
    function made(output, row, key) {
        return ready.thing(row, key, {
            thing: true,
            keywords: output.name,
            stays: false,
            wordsId: undefined,
            targets: undefined,
            verbs: () => provider.verbs(output),
            drawer: () => provider.drawerOf(key),
            options: provider.lineOptions(output)
        });
    }

    // A lit screen's settings as :setl takes them, as the windows daemon
    // serves them on its row (line.rs): scale=1.5, refresh=59.951,
    // resolution=2560x1440, rotation=upright and variablerefresh, each
    // completing from its screen's own choices with their words beside
    // and saying where it stands. What's typed is the daemon's to find,
    // and to refuse in words saying what the screen offers, so a set says
    // nothing at once and the answer comes later. On every screen, power
    // and unplugged do the way the daemon serves for them (lineWay).
    function lineOptions(output) {
        const key = screenKey(output);
        const line = name => ready.served(key)?.held?.line?.[name] ?? null;
        const options = {};
        for (const [name, served] of Object.entries(ready.served(key)?.held?.line ?? {})) {
            const set = served.ways
                ? to => lineWay(key, line(name)?.ways?.[to ? "on" : "off"])
                : (to, change, later) => lineAsk(output, name, to, later);
            options[name] = served.kind === "switch"
                ? { kind: "switch", now: () => line(name)?.now === true, set: set }
                : { kind: "choice", choices: () => line(name)?.values ?? [],
                    now: () => line(name)?.now ?? null, set: set };
        }
        return options;
    }

    // Asks the windows daemon to set what the line typed, a switch's on or
    // off and a choice's words, and hands `later` the words it answers
    // with where it wasn't set: the daemon's own, or the launcher's for an
    // answer that never came. Gone, the daemon can't be asked, so it says
    // so at once.
    function lineAsk(output, option, to, later) {
        const becoming = Words.field.becoming ?? {};
        if (!ready.link)
            return becoming.noAnswer ?? "";
        const typed = typeof to === "boolean" ? to : String(to);
        ready.link.ask({ ask: "screen", do: "line", output: output.name, option, typed },
            answer => later?.(!answer ? becoming.noAnswer ?? ""
                : answer.error ? answer.error.said || (becoming.couldNot ?? "") : "", true));
        return "";
    }

    // What :setl power or :setl unplugged does on the screen of row `key`,
    // as the daemon serves the `way` (line.rs): the act its Turn It Off,
    // Turn It On or Unplug It row does, asked as Return asks it, so the
    // last lit screen's darkening closes the launcher first; or the words
    // said in its place, as unplugging the last lit screen's are.
    function lineWay(key, way) {
        if (way?.said !== undefined)
            return way.said;
        const act = (ready.served(key)?.acts ?? []).find(one => one.key === way?.act);
        if (act)
            ready.act(key, act);
        return "";
    }

    function screenKey(output) {
        return `row:displays:display:${output.name}`;
    }

    // Every screen off, as niri's power-off-monitors turns them off: each
    // stays connected and enabled, so the machine still counts as docked
    // and a monitor's hub and network card stay powered, which taking it
    // out of niri's outputs doesn't keep (Joel, 2026-10-24: "is there a way
    // to turn my external display off without shutting down its hub or
    // network card or anything? i want the laptop to be able to keep going
    // without blinding me in my sleep"). Any key or the pointer wakes them.
    // Its row and its act are the daemon's, served while a screen is lit,
    // which turns them off once the key that asked is up, so Return closes
    // the launcher first (modules/windows, screens/acts.rs).
    function sleepRow(row) {
        return ready.thing(row, "row:displays:sleep", {
            stays: false,
            targets: undefined,
            verbs: null
        });
    }

    // What the vocabulary says of a screen, by the part it's of and its
    // own word: displays:modes' "preferred".
    function partSaid(id, key) {
        return String((Words.of(id === "" ? "displays" : `displays:${id}`).said ?? {})[key]
            ?? key);
    }

    // The screen whose drawer shows, by its row's key, which is watched
    // ahead while it does (ready.ahead), so its light is read; set as the
    // turn ends, never inside the drawer's own binding, as Agents' is.
    property string drawerWanted: ""

    // A screen's drawer as the windows daemon serves it on its row: its
    // connector over its name, its light where it holds the Brightness,
    // what niri says it runs, then what its EDID says, its serial last
    // (modules/windows, screens.rs and light.rs).
    function drawerOf(key) {
        if (key !== drawerWanted)
            Qt.callLater(() => {
                provider.drawerWanted = key;
            });
        const row = ready.served(key);
        return row ? ready.drawerOf(row) : null;
    }

    readonly property var all: ready.rows()
    onAllChanged: changed()

    // What one thing's spots are made of, so a change to one screen makes
    // only its parts again as the launcher closes (Launcher.qml's
    // heldSpots): its served rows, those inside it among them, its EDID's
    // facts and its ways (ReadyRows.stampOf), a screen's saying whether
    // it holds the light; never a level. Joel's closing at 14:36 on
    // 2026-11-11 made all 87 of Displays' rows again.
    function spotsStampOf(thing) {
        return ready.stampOf(thing);
    }

    // What its spots are made of (Launcher.qml's spotsFrom): the rows as
    // the daemon places them, which move with each change it tells, and
    // which screen holds the Brightness.
    property string spotsStamp: JSON.stringify([ready.placings, backlitKey])

    // A display's right arrow: what the daemon serves inside its row
    // (screens.rs), as a step in offers it: on the lit screen holding the
    // light, its Brightness and Room Light, then a lit one's
    // settings, each saying how it's set and stepping into its choices
    // (which what's typed finds from here, so "scale 1.5" is Scale's
    // 1.5\u00d7), then the ways it turns off or on, then what its EDID
    // says, Return copying one.
    function verbs(output) {
        return ready.inside(screenKey(output));
    }

    // Asked of a display by a part's words, "colour depth" or "refresh
    // rate": the focused one's, or each one's when several are on.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["brightness", "warmth", "room", "scale", "refresh", "resolution",
            "rotation", "depth", "colours", "white", "primaries", "hdr"])
            for (const question of Words.answers(`displays:${id}`))
                out[question] = id;
        return out;
    }
    questions: Object.keys(partsAsked)

    // What sets the brightness: "brightness 40", or "brightness to 40%".
    readonly property var brightnessTyped: /^brightness\s+(?:to\s+)?(\d{1,3})\s*%?$/

    function answer(typed) {
        const set = brightnessTyped.exec(typed.trim().toLowerCase());
        if (set)
            return setTo(Math.min(100, Number(set[1])));
        const id = partsAsked[asks(typed)];
        if (id === undefined)
            return null;
        if (id === "warmth")
            return warmthAnswer();
        // The windows daemon serves a row for each lit screen with the
        // part, the focused one's first (modules/windows,
        // screens/answers.rs): the first's part says how it stands, and
        // the rest are listed below it.
        const answering = `row:displays:answer:${id}`;
        const answers = ready.keys.filter(key => ready.served(key)?.parent === answering);
        const first = answers.length > 0 ? ready.served(answers[0]) : null;
        const part = first !== null ? ready.rowAt(first.held.part) : null;
        if (part === null)
            return null;
        const others = answers.slice(1).map(key => ready.rowAt(key)).filter(row => row !== null);
        const light = id === "brightness" || id === "room";
        return {
            // While the windows daemon reads the light, in its first
            // second, a light's answer is still being asked, so Return
            // waits for it; while it shows, it's asked again as the light
            // moves (lightSaid).
            asking: light && servedBrightness !== null && servedBrightness.share === null
                && servedBrightness.said === "" && ready.state !== "not answering",
            live: light ? provider : null,
            // A switch says how it stands in full, as its note does.
            headline: part.said ?? part.side,
            glyph: Theme.glyph.display,
            note: first.name,
            rows: others.length === 0 ? (part.verbs ? part.verbs() : []) : others
        };
    }

    // "screen warmth": the Warmth, Screens' own beside the screens, as
    // its row says it, its switches and figures below; nothing while it
    // isn't served.
    function warmthAnswer() {
        const warmth = all.find(row => row.key === "displays:warmth");
        if (warmth === undefined)
            return null;
        return {
            headline: warmth.side,
            glyph: Theme.glyph.display,
            note: warmth.note,
            rows: warmth.verbs ? warmth.verbs() : []
        };
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
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }

    // How many screens there are and how many are on, as the windows
    // daemon's summary says it beside their rows (screens.rs).
    function counted() {
        void ready.placings;
        return String(ready.held["summary:displays"]?.summary?.said ?? "");
    }
}
