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
// and niri's record of it, which its acts read here; the daemon asks
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
// shell reads no file for them.
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
// 40" sets it on Return. The
// backlight and the room's light are the machine daemon's (modules/machine,
// light.rs), read once a second while they can be seen and told here as
// they move, so the shell reads no file for them, and the sensor's slow
// count is the daemon's to wait on, on a thread of its own. The screen the backlight's device
// names as its connector holds Brightness, and is never said, since its
// drawer's eyebrow already names it.
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
// warmth ask what its switches ask (switchOption).
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../Hearing.js" as Hearing
import "Backlight.js" as Backlight
import "Order.js" as Order
import "ScreenAnswers.js" as ScreenAnswers
import "Screens.js" as Screens
import "Seen.js" as Seen

Provider {
    id: provider

    what: "display"

    collection: Words.collection("displays")

    // The screens' rows as the windows daemon serves them, each holding
    // niri's record of its screen and its drawer; Turn Every Screen Off is
    // the daemon's row too, while one is lit. The screen whose drawer
    // shows is watched ahead, and the one holding the Brightness while the
    // lights can be seen, as by words asking the Room Light, or a terminal
    // looks, so the daemon reads its light from the machine daemon only
    // then (modules/windows, light.rs).
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        place: "displays"
        ahead: [provider.drawerWanted,
            provider.lightsSeen || provider.terminalLooks ? provider.backlitKey : ""]
            .filter((key, at, keys) => key !== "" && keys.indexOf(key) === at)
        make: (row, key) => provider.madeFrom(row, key)
        maker: provider
        drawn: provider.launcher?.open ?? false
    }

    // What niri says of each display, by its connector's name, as its
    // served row holds it.
    readonly property var outputs: {
        void ready.placings;
        const out = {};
        for (const key of ready.keys) {
            const held = ready.served(key)?.held;
            if (held?.niri)
                out[held.output] = Object.assign({ name: held.output }, held.niri);
        }
        return out;
    }

    // :set brightness=60 sets the screen as its Brightness levels do, asking
    // the windows daemon what they ask (brighten), on a machine with a
    // backlight; :set nightlight and nonightlight turn the night light on
    // and off, and :set nowarmth gives the screens their own white by day,
    // as the Warmth's switches do, while each is served. :set
    // noscreens turns every screen off, doing Turn Every Screen Off's act;
    // they are on whenever the launcher can be seen, so screens alone does
    // nothing.
    setOptions: Object.assign({
        screens: {
            kind: "switch",
            now: () => true,
            set: to => {
                const sleep = ready.served("row:displays:sleep")?.acts?.[0];
                if (!to && sleep)
                    ready.act("row:displays:sleep", sleep);
                return "";
            }
        }
    }, light === null ? {} : {
        brightness: {
            kind: "number",
            least: 0,
            most: 100,
            now: () => provider.light?.share ?? 0,
            set: (to, change, later) => {
                provider.brighten(to, later);
                return "";
            }
        }
    }, switchOption("nightlight", "displays:night-light"),
        switchOption("warmth", "displays:follow-room"))

    // The rows served, placed or not, for what the : line asks of them.
    function servedRows() {
        return ready.keys.map(key => ready.served(key));
    }

    // The : line's option `name` for the served switch whose act is `act`,
    // offered only while it's served: it says how the switch stands, and
    // turning it the other way asks what its Return asks (Screens.js's
    // switchOf).
    function switchOption(name, act) {
        if (Screens.switchOf(servedRows(), act) === null)
            return {};
        return { [name]: {
            kind: "switch",
            now: () => Screens.switchOf(provider.servedRows(), act)?.on ?? false,
            set: (to, change, later) => {
                const ask = Screens.switchOf(provider.servedRows(), act)?.asked(to) ?? null;
                if (ask !== null)
                    provider.lineAsked(ask, later, name);
                return "";
            }
        } };
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

    // The lights are read once as the shell starts too, though the
    // launcher is closed, since what's inside a screen is made from what's
    // held while it's closed (Daemon.qml's refresh).
    Component.onCompleted: lights.refresh()

    function builtIn(output) {
        return output.name.startsWith("eDP");
    }

    // Whether a monitor's brightness follows the panel's, which the
    // windows daemon holds on its row from desk-facts, by a brightness
    // read back from it (screens/facts.rs).
    function follows(output) {
        return ready.served(screenKey(output))?.held?.follows === true;
    }

    // Whether the room's light is heard here: the built-in screen's lid
    // holds the sensor, and so does the only screen there is, as in a
    // virtual machine's.
    function inTheRoom(output) {
        return builtIn(output) || shown.length === 1;
    }

    // Whether this screen's place holds the Brightness: the built-in one
    // while it's on, the only screen there is, or else the first lit
    // monitor that follows the panel. Only one holds it, since every
    // screen shows the same level.
    function backlit(output) {
        const named = light?.connector ?? null;
        const home = lit.find(each => each.name === named)
            ?? lit.find(each => inTheRoom(each))
            ?? lit.find(each => follows(each)) ?? null;
        return home !== null && home.name === output.name;
    }


    // Sets the screen to a share, for :set brightness= and "brightness
    // 40", asking the windows daemon what its Brightness levels ask, at
    // that share (Screens.js's brightening), so it goes as a level's
    // Return does; nothing is asked while no level is served.
    function brighten(share, later) {
        const ask = Screens.brightening(servedRows(), share);
        if (ask !== null)
            lineAsked(ask, later, `brightness ${ask.share}`);
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

    // Setting the screen to a share, Return doing it.
    function setTo(share) {
        const light = provider.light;
        if (light === null)
            return null;
        const kept = Math.max(1, share);
        return {
            headline: partSaid("", "setTo").replace("{share}", kept),
            verb: Words.command("command:brightness").verb,
            stays: true,
            run: () => provider.brighten(kept),
            rows: [],
            note: lightRead ? partSaid("", "now").replace("{share}", light.share)
                : unreadSaid()
        };
    }

    // Whether a light's answer shows, out at the top, which the launcher
    // asks again as the light moves, so what follows the room moves as it
    // does.
    readonly property bool lightShown: launcher !== null && launcher.open
        && (launcher.held ?? []).length === 0 && launcher.answer?.live === provider

    // The machine daemon's backlights and light sensors, read once a second
    // while they can be seen (lightsSeen); none is held while the daemon
    // isn't answering, and what an earlier watch left is said as not read
    // yet (lightRead), so no old level is said as now.
    Daemon {
        id: lights

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["backlight", "light", "brightness"]
        live: provider.lightsSeen
        keptThroughGap: false
        answersWhole: true
    }

    // The backlight and the room's sensor as last told (Backlight.js's lit
    // and sensor), for a screen's drawer and the rows a step into it holds,
    // each replaced only when what it says changed, so the room's light
    // moving leaves the backlight be; and `room`, the lux the sensor hears,
    // or null until it's said one.
    property var light: null
    onLightChanged: changed()
    // Whether what's held of the lights was told for this watch, so says
    // the level as it is now; until then the level held from an earlier
    // watch is said as not read yet, in the same rows.
    readonly property bool lightRead: lights.answeredThisWatch
    // What a level not yet read says: nothing for its first second, which
    // the daemon's answer nearly always beats, so no word flashes past,
    // then that it's not read yet (VKCF3Z, 2026-11-10). The sensor's count
    // is read on a thread of its own, so it can come after the backlight's.
    function unreadSaid() {
        return lightWaited ? partSaid("", "unread") : "";
    }
    property bool lightWaited: false
    onLightWaitedChanged: changed()
    readonly property bool lightsAllRead: lightRead && (roomSensor === null || room !== null)
    // Fires a second into a watch that hasn't read everything, and each
    // second after, which changes nothing; it repeats so that its running
    // stays bound, since a timer that stops itself lets go of its binding.
    Timer {
        interval: 1000
        repeat: true
        running: lights.live && !provider.lightsAllRead
        onTriggered: provider.lightWaited = true
    }
    property var roomSensor: null
    onRoomSensorChanged: changed()
    readonly property var room: roomSensor?.lux ?? null

    Connections {
        target: lights

        function onRevisionChanged() {
            provider.heard();
        }

        // A light's answer waits for the watch's first answer, so it's
        // asked again once that's come, moved or not. Asked as the turn
        // ends, never at once: a watch ending as lightsSeen changes says
        // so here, inside lightsSeen's own binding, and an answer asked
        // there makes the rows lightsSeen reads again, a binding loop
        // whenever the watch was answered before the next key (the
        // answers VM test, 2026-11-10).
        function onAnsweredThisWatchChanged() {
            Qt.callLater(Hearing.putOff(provider.lightTold));
        }

        function onLiveChanged() {
            provider.lightWaited = false;
        }
    }

    // Timed from the line that answered the watch (Hearing.js).
    readonly property var lightTold: Hearing.carried(() => provider.lightAnswered())
    function lightAnswered() {
        if (lightShown)
            answered();
    }

    // Each of what the lights say is replaced only when it changed, so the
    // room's light moving leaves the rest be.
    function heard() {
        const records = Object.values(lights.held);
        const now = Backlight.lit(records);
        const sensed = Backlight.sensor(records);
        const moved = (was, is) => JSON.stringify(was) !== JSON.stringify(is);
        const lit = moved(light, now);
        const sensing = moved(roomSensor, sensed);
        if (lit)
            light = now;
        if (sensing)
            roomSensor = sensed;
        if ((lit || sensing) && lightShown)
            answered();
    }

    // Whether a terminal's `launcher show` walked into Screens or to a
    // screen (Launcher.qml's terminalSees), so its drawer's light is read
    // with the launcher closed too.
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.from === provider || String(row?.key ?? "").startsWith("display:"))
    // A terminal's look waits for the daemon's whole answer to its watch,
    // as the launcher shows the light once it opens (Provider.stillReading).
    // Its drawer waits too, while a backlight is told, for the daemon to say
    // its Brightness, read or not yet read, which it does within a second
    // of its look (modules/windows, light.rs's WAITED).
    stillReading: terminalLooks && !(launcher?.open ?? false) && lights.state !== "not answering"
        && (!lights.answeredThisWatch || light !== null && backlitKey !== ""
            && brightnessServed(backlitKey) === "")

    // The row's key of the screen whose place holds the Brightness, or "".
    readonly property string backlitKey: {
        const home = lit.find(output => backlit(output));
        return home ? `row:displays:display:${home.name}` : "";
    }

    // What the drawer the daemon serves on the row of `key` says of the
    // Brightness, "" while it says none.
    function brightnessServed(key) {
        void ready.placings;
        const named = partName("brightness");
        const fact = (ready.served(key)?.drawer?.facts ?? []).find(each => each.name === named);
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

    // Its maker and model, leaving out a model that's only a number.
    function named(output) {
        if (builtIn(output))
            return "Built-in screen";
        const model = /^0x[0-9a-f]+$/i.test(output.model) ? "" : output.model;
        return `${output.make} ${model}`.trim() || output.name;
    }

    // Screens in the order the daemon serves them: the built-in first, as
    // it's always there, then by connector.
    readonly property var shown: Object.values(outputs)
    readonly property var lit: shown.filter(output => output.logical !== null)

    readonly property string focusedOutput: {
        void Niri.revision;
        const workspace = Niri.workspace();
        return workspace ? workspace.output : "";
    }

    // What a served row is made into: a screen's, by the niri record it
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
        const output = outputs[row.held?.output];
        return output ? made(output, row, key) : null;
    }

    // A screen's row as the daemon says it, its name, how it stands and
    // what it runs, and its Return, focusing it or turning it on, the
    // daemon's act (modules/windows, screens/acts.rs). Its verbs are its
    // light, made here, and what the daemon serves inside it, so Return,
    // done, closes the launcher as it did, and the key language reads none
    // of its act.
    function made(output, row, key) {
        const stands = Screens.standing(output);
        return ready.thing(row, key, {
            thing: true,
            keywords: output.name,
            stays: false,
            wordsId: undefined,
            targets: undefined,
            verbs: () => provider.verbs(output),
            drawer: () => provider.drawerOf(key),
            options: Object.assign({
                power: provider.screenOption(output, "power"),
                unplugged: provider.screenOption(output, "unplugged")
            }, provider.lineOptions(output))
        });
    }

    // A lit screen's settings as :setl takes them, as the windows daemon
    // serves them on its row (line.rs): scale=1.5, refresh=59.951,
    // resolution=2560x1440, rotation=upright and variablerefresh, each
    // completing from its screen's own choices with their words beside
    // and saying where it stands. What's typed is the daemon's to find,
    // and to refuse in words saying what the screen offers, so a set says
    // nothing at once and the answer comes later.
    function lineOptions(output) {
        const key = screenKey(output);
        const line = name => ready.served(key)?.held?.line?.[name] ?? null;
        const options = {};
        for (const [name, served] of Object.entries(ready.served(key)?.held?.line ?? {})) {
            const set = (to, change, later) => lineAsk(output, name, to, later);
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

    // A screen's option for the : line's :setl, which does the act its
    // Turn It Off, Turn It On or Unplug It row does (Screens.wayTo); one it
    // can't take, as unplugging the last lit screen, says so and does
    // nothing else.
    function screenOption(output, option) {
        return {
            kind: "switch",
            now: () => Screens.optionNow(output, option),
            set: on => {
                const key = screenKey(output);
                const acts = ready.served(key)?.acts ?? [];
                const asked = { option: option, on: on };
                const told = Screens.toldOf(output, acts, asked);
                if (told !== "")
                    return ScreenAnswers.toldSaid(told, named(output), screenWords);
                ready.act(key, Screens.wayTo(output, acts, asked));
                return "";
            }
        };
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

    // The vocabulary as ScreenAnswers.js says a screen in it.
    readonly property var screenWords: ({
        said: (id, key) => provider.partSaid(id, key)
    })

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
    // facts and its ways (ReadyRows.stampOf), and for a screen whether it
    // holds the light and the sensor; never a level. Joel's closing
    // at 14:36 on 2026-11-11 made all 87 of Displays' rows again.
    function spotsStampOf(thing) {
        const served = ready.stampOf(thing);
        const name = served === null ? undefined
            : ready.served(ready.madeOf[thing.key])?.held?.output;
        const output = name === undefined ? undefined : outputs[name];
        if (output === undefined)
            return served;
        return JSON.stringify([served, backlit(output),
            light === null ? false : light.connector ?? "", roomSensor !== null]);
    }

    // What its spots are made of (Launcher.qml's spotsFrom): the screens
    // as the daemon serves them, which one holds the Brightness, and
    // whether the room's sensor is there; never a level or a lux, which
    // move while nothing else does.
    property string spotsStamp: JSON.stringify([ready.placings,
        light === null ? false : light.connector ?? "", roomSensor !== null])

    // A part's name, as the vocabulary says it (Provider's partNamed).
    function partName(id) {
        return partNamed("displays", id);
    }

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
        if (id === undefined || lit.length === 0)
            return null;
        if (id === "warmth")
            return warmthAnswer();
        const partOf = output => verbs(output).find(row => row.name === partName(id)) ?? null;
        // The focused screen's, or the first that has it: only one has a
        // Brightness.
        const focused = lit.find(output => output.name === focusedOutput && partOf(output))
            ?? lit.find(output => partOf(output) !== null);
        if (focused === undefined)
            return null;
        const part = partOf(focused);
        const others = lit.filter(output => output !== focused && partOf(output) !== null);
        const light = id === "brightness" || id === "room";
        return {
            // Until the daemon's first answer comes, a light's answer is
            // still being asked, so Return waits for it; while it shows,
            // it's asked again as the light moves.
            asking: light && !lights.answeredThisWatch && lights.state !== "not answering",
            live: light ? provider : null,
            // A switch says how it stands in full, as its note does.
            headline: part.said ?? part.side,
            glyph: Theme.glyph.display,
            note: named(focused),
            rows: others.length === 0 ? (part.verbs ? part.verbs() : [])
                : others.map(output => copyRow({
                    key: `displays:${output.name}:${id}:said`,
                    name: named(output),
                    side: partOf(output).side,
                    copy: "",
                    glyph: ""
                }))
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

    function counted() {
        const each = shown.length === 1 ? "screen" : "screens";
        return lit.length === shown.length ? `${shown.length} ${each}`
            : `${lit.length} of ${shown.length} on`;
    }
}
