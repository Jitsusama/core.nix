// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed in Now's groups, what wants Joel first,
// then what reaches other people, what's playing and the modes left on, and
// by name within each (Now.js), so what can need him leads and one keeps
// its place while its group stands; the time, the date, the weather and
// the next meeting are always last, Now's own rows. What each asks
// of Joel shows in its glyph's colour too. Return does a piece's first
// action, and the
// right arrow steps into the rest.
//
// It holds nothing of its own: the pieces are the margin's (Facts.qml), so
// a glyph and its row come and go together, and it adds no row to a search.
// It's home's first row, Now, saying what's in it by name.
import qs
import Quickshell
import QtQuick
import "../Marks.js" as Marks
import "Now.js" as Now

Provider {
    id: provider

    what: "on"

    // Named Now, as the vocabulary says (Words.qml), under the calendar
    // clock.
    collection: Words.collection("on", { searched: false })
    // What waits on Joel is found by its words too, since Now is its one
    // home: the rest of what's on each has a place of its own to be found
    // in, which is why the collection adds no row to a search.
    things: (waiting === null ? [] : [waiting]).concat([location])

    // What the thing is where the launcher has a row of its own for it, by
    // its mark, so one found in both is listed once (Rank.identity).
    readonly property var same: ({
        awake: "session:awake",
        quiet: "session:quiet",
        saver: "session:profile",
        recording: "session:recording"
    })

    readonly property var all: {
        const lit = Facts.all.filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active);
        // By group, what wants Joel first, and by name within each, so a
        // row keeps its place while its group stands (Now.js); this
        // moment's rows, its own, always last.
        const rows = lit.map(fact => ({
            group: fact.piece.group,
            name: fact.piece.title,
            row: made(fact)
        }));
        if (waiting !== null)
            rows.push({ group: "wants", name: waiting.name, row: waiting });
        return Now.ordered(rows).map(each => each.row).concat(moment);
    }
    onAllChanged: changed()

    // This moment, Now's own rows, always its last, after what's on: the
    // Time, the Date, the Weather here and the next meeting today, each
    // a row of Now itself rather than inside a Today to step into (Joel,
    // 2026-10-28: "what is the difference between top level now and
    // today? it feels incongruent ... i want time in now also"). Each is
    // that thing's one row, the same wherever it shows (Joel, 2026-10-28:
    // "in now, time and date should be their fully interactive version
    // ... they need to be treated the same everywhere"): the time, the
    // date and the weather as the launcher places an answer
    // (Launcher.qml's resolved), named by the thing, what it says beside,
    // its parts a step in and Return copying it; the meeting as the
    // calendar provider's own row (`next`), whose Return joins it, or
    // steps into it where it can't be joined yet. A machine without the
    // calendar, or a day with no meeting left, has no meeting row, and one
    // that can't say the weather has none of it.
    readonly property var calendar: source("calendar")
    readonly property var weatherSource: source("weather")
    readonly property var meeting: calendar?.next ?? null

    function source(name) {
        return (provider.launcher?.sources ?? []).find(one => one.settings.name === name) ?? null;
    }

    // The weather here, as its answer says it, kept from one asking to the
    // next: read when the launcher opens, so it's there before Joel steps
    // into Now, and again whenever the weather hears from Open-Meteo. Its
    // answer asks again only once what it holds is a quarter of an hour
    // old, so opening often asks nothing.
    property var weather: null

    function reweather() {
        const row = weatherSource ? provider.launcher?.resolved?.("weather") ?? null : null;
        // Still asking, failed, or not knowing where here is, it keeps what
        // it last knew rather than blinking out.
        if (row)
            weather = row;
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open) {
                provider.reweather();
                provider.retime();
            }
        }
    }

    // The time and the date as the launcher places them, read rather than
    // bound: the launcher's answers are made from every provider's rows,
    // Now's among them, so a binding here would read itself back (a loop
    // Quickshell warns of as it starts). Read as the clock turns a minute,
    // as the launcher opens, and once the shell has started.
    property var timeRow: null
    property var dateRow: null

    function retime() {
        const placed = id => provider.launcher?.resolved?.(id) ?? null;
        timeRow = placed("time");
        dateRow = placed("today");
    }

    Connections {
        target: clock

        function onDateChanged() {
            Qt.callLater(provider.retime);
        }
    }

    Component.onCompleted: Qt.callLater(retime)
    onLauncherChanged: Qt.callLater(retime)

    Connections {
        target: provider.weatherSource

        function onAnswered() {
            provider.reweather();
        }
    }

    // The time to the minute, which the Time row says, read as the clock
    // turns, never polled.
    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    // Where the laptop is, as Location says it, the one place the weather
    // and everything else that depends on it reads here from (Joel,
    // 2026-10-28: "a location area in now ... tied to what weather and
    // any other geo-conscious options pull from"): its town, else its
    // position, and how sure it is. Inside, the position, how it was
    // found, and the time zone the clock keeps, saying so where the place
    // keeps another. Found by typing its name too, since Now is its home.
    // Once the clock has followed the laptop into another zone, it says so
    // and Put It Back leads what's inside (Joel, 2026-10-28: "i really
    // want my timezone to automatically change if possible").
    readonly property var location: {
        const here = Location.here;
        const position = here ? `${here.latitude.toFixed(2)}, ${here.longitude.toFixed(2)}` : "";
        const within = here ? `within ${Math.ceil(here.accuracy / 1000)} km` : "";
        const side = here ? here.place ?? position : Location.read ? "unknown" : "\u2026";
        const differs = Location.zoneDiffers;
        // Said plainly where it's unknown, never guessed.
        const elsewhere = differs ? `it keeps ${here.zone}, not the clock's ${Location.clockZone}`
            : "";
        const moved = Location.clockMoved ? Location.moved : null;
        // "Clock moved to Edmonton time, from Calgary": the zone by its
        // city, and where the laptop was, by its town.
        const city = zone => zone.split("/").pop().split("_").join(" ");
        const said = moved ? `clock moved to ${city(moved.to)} time`
            + (moved.place ? `, from ${moved.place.split(",")[0]}` : "") : "";
        const note = !here ? (Location.read ? "GeoClue hasn't found the laptop yet" : "")
            : moved ? said : differs ? elsewhere : `${within}, found through GeoClue`;
        const inside = () => {
            const rows = [];
            if (moved)
                rows.push(thing({
                    key: "verb:location:put-back",
                    verb: Words.verb("verb:location:put-back"),
                    name: Words.name("verb:location:put-back"),
                    glyph: Words.glyph("verb:location:put-back"),
                    side: city(moved.from),
                    hue: "action",
                    stays: true,
                    run: () => Location.putBack()
                }));
            if (here)
                rows.push(copyRow({
                    key: "location:position",
                    name: Words.name("location:position"),
                    side: position,
                    note: within
                }));
            rows.push(copyRow({
                key: "location:zone",
                name: Words.name("location:zone"),
                side: Location.clockZone,
                note: elsewhere,
                hue: differs ? "strong" : "voice"
            }));
            return rows;
        };
        return thingRow({
            key: "on:location",
            name: Words.name("location"),
            side: side,
            note: note,
            hue: moved || differs ? "strong" : "voice",
            glyph: Theme.glyph.location
        }, inside);
    }

    // Made again as the clock turns a minute, so the time says it.
    readonly property var moment: {
        const rows = [timeRow, dateRow, location];
        if (weather)
            rows.push(weather);
        if (meeting)
            rows.push(meeting);
        return rows.filter(row => row !== null);
    }

    function waitingSaid(id, fill) {
        let words = (Words.of("on:waiting").said ?? {})[id] ?? id;
        for (const key of Object.keys(fill))
            words = words.replace(`{${key}}`, fill[key]);
        return words;
    }

    // How long since an item was last said, for one gone stale: in hours,
    // and in days past one.
    function since(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    // Where an item is done, by name: the session its window is, else the
    // window's own title; "" when none is open.
    function whereOf(each) {
        const window = WaitingOnYou.windowOf(each);
        if (window === null)
            return "";
        const session = Sessions.running.find(one => one.window.id === window.id);
        return session ? session.title : window.title ?? "";
    }

    // An act on an item, named in the vocabulary.
    function act(each, id, fields) {
        return thing(Object.assign({
            key: `verb:on:waiting:${id}:${WaitingOnYou.identity(each)}`,
            verb: Words.verb(`verb:on:waiting:${id}`),
            name: Words.name(`verb:on:waiting:${id}`),
            glyph: Words.glyph(`verb:on:waiting:${id}`),
            hue: "action"
        }, fields));
    }

    // What there is to know of an item, each a part named first, saying its
    // value and copying it: why it waits, when, whose quest, since when it's
    // been said, and the detail its owner gave.
    function facts(each) {
        const fact = (id, value) => typeof value !== "string" || value === "" ? null
            : copyRow({
                key: `fact:on:waiting:${id}:${WaitingOnYou.identity(each)}`,
                name: Words.name(`on:waiting:${id}`),
                side: value,
                copy: value,
                glyph: Theme.glyph.waitsOnJoel
            });
        return [
            fact("why", each.why),
            fact("when", each.when),
            fact("quest", WaitingOnYou.questOf(each)),
            fact("since", Number.isNaN(each.since) ? "" : since(each.since)),
            fact("detail", each.detail)
        ].filter(row => row !== null);
    }

    // One thing waiting on Joel, as the agents keep them (WaitingOnYou.qml):
    // its title, how long it takes, and why it waits. Return goes to the
    // window it's done in, or, with none open, steps in, so it's never a
    // key that does nothing. One he can do now is strong and says its
    // minutes; one for later is muted and says when, in its owner's words.
    // The right arrow steps into it: going to its window, then Done and Not
    // Now, which stay open as the item leaves its place, then copying it,
    // never first (Joel, 2026-10-28: "having the separate copy option at the
    // top is very confusing"), then what there is to know of it.
    function item(each) {
        const now = each.ready && !each.putOff;
        const where = whereOf(each);
        const does = where === "" ? null : act(each, "go", {
            name: Words.name("verb:on:waiting:go").replace("{where}", where),
            later: true,
            run: () => WaitingOnYou.go(each)
        });
        const inside = () => [does,
            act(each, "done", { stays: true, run: () => WaitingOnYou.settle(each, "done") }),
            act(each, "notNow", { stays: true, run: () => WaitingOnYou.settle(each, "notNow") }),
            // The clipboard takes text only while the launcher holds the
            // keys, so a copy is made before it closes.
            act(each, "copy", {
                now: true,
                run: () => {
                    Quickshell.clipboardText = WaitingOnYou.titled(each);
                }
            })
        ].filter(row => row !== null).concat(provider.facts(each));
        const row = member({
            key: `on:waiting:${WaitingOnYou.identity(each)}`,
            verb: does?.verb ?? "more",
            stays: does === null,
            later: does?.later ?? false,
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: now ? "strong" : "muted",
            side: !now && typeof each.when === "string" ? each.when
                : typeof each.minutes === "number"
                ? waitingSaid("minutes", { minutes: each.minutes }) : "",
            note: typeof each.why === "string" ? each.why : "",
            verbs: inside,
            // With nowhere to go, Return steps in, as the right arrow does:
            // its verbs and targets both make it a row holding rows, which
            // Return's own act never leads (Catalogue.js's leading).
            targets: does === null ? inside : null,
            run: () => does === null ? provider.launcher?.enter(row) : does.run()
        });
        return row;
    }

    // What the agents have waiting on Joel, one row however many: it counts
    // and colours only what he can do now, its note names the first of
    // those, else the first for later, which Return goes to, and the right
    // arrow lists what's for now and then what's for later, each in the
    // agents' own order. With only later things, the row is muted and
    // says how many; with nothing waiting, no row.
    readonly property string first: WaitingOnYou.foremost
        ? WaitingOnYou.titled(WaitingOnYou.foremost) : ""
    readonly property var lead: WaitingOnYou.foremost === null ? null
        : provider.item(WaitingOnYou.foremost)
    readonly property var waiting: lead === null ? null : member({
        // Not `on:waiting`, which is the missed notices' row, by its mark
        // (pieces/Waiting.qml), so the two are never taken for one.
        key: "on:waitingOnYou",
        verb: lead.verb,
        stays: lead.stays ?? false,
        later: lead.later,
        name: Words.name("on:waiting"),
        glyph: Theme.glyph.waitsOnJoel,
        hue: WaitingOnYou.now.length > 0 ? "strong" : "muted",
        side: WaitingOnYou.now.length > 0 ? String(WaitingOnYou.now.length)
            : waitingSaid("later", { count: WaitingOnYou.later.length }),
        note: first,
        // Its rows are things with verbs of their own, so the right arrow
        // steps into one and Return still does its act; a row giving
        // targets would step in on Return instead.
        nested: true,
        verbs: () => WaitingOnYou.items.map(each => provider.item(each)),
        // Where the first has nowhere to go, Return steps into this row's
        // list, as the right arrow does, rather than into the first alone.
        run: () => provider.lead.verb === "more" ? provider.launcher?.enter(provider.waiting)
            : provider.lead.run()
    })
    // What home's Now leads with: the first thing waiting, after its row's
    // own name, or nothing.
    readonly property string foremost: first === "" ? ""
        : `${Words.name("on:waiting")}: ${first}`

    // The launcher, as a piece's actions are given it, so one that opens a
    // collection opens it here.
    readonly property var shell: ({ launcher: provider.launcher })

    function made(fact) {
        const piece = fact.piece;
        // Every act a piece has, where it keeps more than its notice can
        // offer (pieces/Pi.qml's every).
        const actions = (piece.every ?? piece.actions).map(action => thing({
            key: `verb:on:${fact.entry.mark}:${action.name}`,
            name: action.name,
            glyph: action.glyph,
            hue: "action",
            chord: action.chord ?? [],
            // One that changes how things stand, such as Let It Sleep,
            // keeps the launcher, which shows it changed (Piece.qml).
            stays: action.stays ?? false,
            run: () => action.run(provider.shell)
        }));
        // A piece standing for things of the launcher's own, such as the
        // mail that wants Joel, lists them on Return, each the row it is
        // everywhere else, and its own acts after them (Piece.qml's rows).
        const holding = piece.rows ? () => piece.rows(thing).concat(actions) : null;
        return member({
            key: `on:${fact.entry.mark}`,
            is: provider.same[fact.entry.mark] ?? `on:${fact.entry.mark}`,
            // What Return does, said in the foot: the first action's own
            // verb where it gives one, else its name ("let it sleep").
            verb: piece.actions.length === 0 ? "close"
                : piece.actions[0].verb ?? piece.actions[0].name.toLowerCase(),
            name: piece.title,
            side: piece.value,
            note: piece.context,
            glyph: Theme.glyph[piece.markGlyph || fact.entry.mark],
            hue: Marks.tone(piece),
            plain: !holding && actions.length === 0,
            stays: !holding && actions.length > 0 && actions[0].stays,
            // Return's act leads the right arrow, then the rest, so every
            // act is there; a piece standing for things holds them instead,
            // and with no act of its own, Return steps in to them too.
            verbs: holding ?? (actions.length > 1 ? () => actions : null),
            run: holding ? null : () => {
                if (actions.length > 0)
                    actions[0].run();
            }
        });
    }

    function everything() {
        return all.slice();
    }

    // This moment's rows are always there, so what's counted is what's on
    // beside them.
    function counted() {
        const on = all.length - moment.length;
        return on === 0 ? "nothing" : String(on);
    }
}
