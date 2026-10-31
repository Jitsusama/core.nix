// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed in Now's groups, what wants Joel first,
// then what reaches other people, what's playing and the modes left on, and
// by name within each (Now.js), so what can need him leads and one keeps
// its place while its group stands. Now's top never changes (docs/design.md,
// home is a few places): what waits on Joel first, then Now's own rows, the
// time, the date, the location, the weather and the next meeting, and only
// then what else is on, which comes and goes below them. What each asks
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
import "../Fuzzy.js" as Fuzzy
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
    things: [waiting, location]

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
            && fact.piece.present && fact.piece.active && above(fact) === "");
        // What waits on Joel, then this moment's rows, which are always
        // there, so the top never moves; then the rest by group, what
        // wants Joel first, and by name within each, so a row keeps its
        // place while its group stands (Now.js).
        const rows = lit.map(fact => ({
            group: fact.piece.group,
            name: fact.piece.title,
            row: own(fact) ?? made(fact)
        }));
        // The sessions there are to reopen, Agents' own row,
        // among what else wants Joel while any are left to reopen (VKCF3Z's
        // design, [fce929da]), drawn the same as there.
        const lost = source("agents")?.toReopen ?? null;
        if (lost)
            rows.push({ group: "wants", name: lost.name, row: lost });
        return [waiting].concat(moment, Now.ordered(rows).map(each => each.row));
    }
    onAllChanged: changed()

    // This moment, Now's own rows, right after what waits on Joel and
    // before what else is on: the
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
    // position, and how sure it is, saying so where the place keeps another
    // zone than the clock. Inside, in the design's order (VKCF3Z's design,
    // "Location. Inside: Town, How It Knows, Set It by Hand, Find It Again,
    // Copy"): its town, how it knows, setting it by hand and finding it
    // again, and Copy, of its position, last. Found by typing its name too,
    // since Now is its home. The zone, and Put It Back after the clock
    // followed it, are Time's (Clocks.qml), since the zone is the clock's.
    readonly property var location: {
        const here = Location.here;
        const position = here ? `${here.latitude.toFixed(2)}, ${here.longitude.toFixed(2)}` : "";
        // A place Joel set himself says so, since GeoClue no longer moves it.
        const byHand = here?.hand === true;
        const within = !here ? "" : byHand ? "set by hand"
            : `within ${Math.ceil(here.accuracy / 1000)} km`;
        const side = here ? here.place ?? position : Location.read ? "unknown" : "\u2026";
        const differs = Location.zoneDiffers;
        // Said plainly where it's unknown, never guessed.
        const elsewhere = differs ? `it keeps ${here.zone}, not the clock's ${Location.clockZone}`
            : "";
        const note = !here ? (Location.read ? "GeoClue hasn't found the laptop yet" : "")
            : differs ? elsewhere : byHand ? within : `${within}, found through GeoClue`;
        const inside = () => {
            const rows = [copyRow({
                key: "location:town",
                name: Words.name("location:town"),
                side: !here ? side : here.place ?? "not named yet"
            }), copyRow({
                key: "location:knows",
                name: Words.name("location:knows"),
                side: !here ? "not found yet" : byHand ? "set by hand"
                    : `GeoClue, ${within}`
            })];
            // Where GeoClue can't find the laptop, or finds it wrong, Joel
            // names its town, and asks GeoClue again once he's moved on.
            rows.push(thing({
                key: "verb:location:set",
                verb: Words.verb("verb:location:set"),
                name: Words.name("verb:location:set"),
                glyph: Words.glyph("verb:location:set"),
                ask: Words.of("verb:location:set").ask ?? "",
                chip: Words.name("location"),
                hue: "action",
                state: () => Location.told,
                given: words => Location.setByHand(words)
            }));
            if (byHand)
                rows.push(thing({
                    key: "verb:location:again",
                    verb: Words.verb("verb:location:again"),
                    name: Words.name("verb:location:again"),
                    glyph: Words.glyph("verb:location:again"),
                    hue: "action",
                    stays: true,
                    run: () => Location.findAgain()
                }));
            // An act to copy, last as Copy always is, of what the town
            // alone can't say: where on a map it is.
            if (here)
                rows.push(copyRow({
                    key: "verb:location:copy",
                    name: Words.name("verb:location:copy"),
                    verb: Words.verb("verb:location:copy"),
                    glyph: Words.glyph("verb:location:copy"),
                    side: position,
                    copy: position,
                    figure: false,
                    plain: false
                }));
            return rows;
        };
        return thingRow({
            key: "on:location",
            name: Words.name("location"),
            side: side,
            note: note,
            hue: differs ? "strong" : "voice",
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

    // A time an item names, as the clock says it: today's by its hour, and
    // another day's with its day.
    function clocked(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    // What one not ready yet waits for, in words (Waiting.waitsFor), or its
    // owner's own words in a file older than that.
    function waitsSaid(each) {
        const waits = WaitingOnYou.waitsFor(each);
        if (waits === null)
            return "";
        if (typeof waits.words === "string")
            return waits.words;
        return waitingSaid(waits.check ? "check" : waits.signal, { value: waits.value ?? "" });
    }

    // Where and by when it's done, each where its owner said, and that
    // nobody's watching it when only Joel can say it's done and the session
    // that raised it has gone (VKCF3Z, 2026-10-30). An item that says none
    // of it says why it waits, as one from a file older than that does.
    function noteOf(each) {
        const time = at => Number.isNaN(Date.parse(at ?? "")) ? "" : clocked(at);
        const parts = [
            time(each.at) === "" ? "" : waitingSaid("at", { time: time(each.at) }),
            time(each.deadline) === "" ? "" : waitingSaid("by", { time: time(each.deadline) }),
            typeof each.where?.machine === "string" && each.where.machine !== ""
                ? waitingSaid("on", { machine: each.where.machine }) : "",
            WaitingOnYou.watched(each) ? "" : waitingSaid("nobody", {})
        ].filter(part => part !== "");
        if (parts.length === 0 && each.v !== 2 && typeof each.why === "string")
            return each.why;
        return parts.join(" \u00b7 ");
    }

    // Its drawer: its steps, numbered, then why only Joel can do it, then
    // how it'll be known done (VKCF3Z, 2026-10-31), the four parts an item
    // says, each only when it says it.
    function drawerOf(each) {
        const steps = (Array.isArray(each.steps) ? each.steps : [])
            .filter(step => typeof step === "string" && step !== "")
            .map((step, at) => `${at + 1}. ${step}`);
        const why = typeof each.why === "string" ? each.why : "";
        const known = knownSaid(each);
        const after = [why, known].filter(part => part !== "").join("\n");
        return { title: each.title, body: steps.concat(after === "" ? [] : ["", after]).join("\n") };
    }

    // How an item will be known done, in words (Waiting.doneBy): by its
    // check or signal, in the phrases what it waits for is said in, or by
    // Joel alone; "" when it doesn't say.
    function knownSaid(each) {
        const done = WaitingOnYou.doneBy(each);
        if (done === null)
            return "";
        if (done.person)
            return waitingSaid("person", {});
        const how = waitingSaid(done.check ? "check" : done.signal, { value: done.value ?? "" });
        return how === "" ? "" : waitingSaid("known", { how: how });
    }

    // Where an item is done, by name: the session its window is, else the
    // window's own title; "" when none is open.
    function whereOf(each) {
        const window = WaitingOnYou.windowOf(each);
        if (window === null)
            return "";
        // A session in no window, as one held or elsewhere is, isn't it.
        const session = Sessions.running.find(one => one.window?.id === window.id);
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

    // One thing only Joel can do, as its owner wrote it (WaitingOnYou.qml,
    // VKCF3Z, 2026-10-30): its title, the act; at its side how long it
    // takes, or, for one not ready yet, what it waits for; in its note where
    // and by when; and in its drawer its steps and why only he can. One he
    // can do now is strong, one for later muted. Return goes where it's
    // done, its window or its session's, and with nowhere to go steps in, as
    // the right arrow does: Go To, then Done and Not Now, which stay open as
    // the item leaves its place, then its quest.
    function item(each) {
        const now = each.ready && !each.putOff;
        const where = whereOf(each);
        // Go To, where it goes at its side, as going to anything open is
        // said everywhere, so the word never changes shape.
        const does = where === "" ? null : act(each, "go", {
            side: where,
            later: true,
            run: () => WaitingOnYou.go(each)
        });
        const quest = () => (source("quests")?.everything() ?? [])
            .find(row => row.key === `quest:${each.quest}`) ?? null;
        const inside = () => [does,
            act(each, "done", { stays: true, run: () => WaitingOnYou.settle(each, "done") }),
            act(each, "notNow", { stays: true, run: () => WaitingOnYou.settle(each, "notNow") }),
            quest()
        ].filter(row => row !== null);
        const row = member({
            thing: true,
            key: `on:waiting:${WaitingOnYou.identity(each)}`,
            verb: does === null ? "more" : "go",
            stays: does === null,
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: now ? "strong" : "muted",
            side: !now && waitsSaid(each) !== "" ? waitsSaid(each)
                : typeof each.minutes === "number"
                ? waitingSaid("minutes", { minutes: each.minutes }) : "",
            note: noteOf(each),
            drawer: () => provider.drawerOf(each),
            // Its verbs and targets both make it a row holding rows, which
            // Return's own act never leads (Catalogue.js's leading).
            verbs: inside,
            targets: inside,
            run: () => does === null ? provider.launcher?.enter(row) : WaitingOnYou.go(each)
        });
        return row;
    }

    // What can't be done yet, one row at the end of the list, so the list
    // holds only what Joel can do now; each inside says what it waits for.
    readonly property var laterRow: WaitingOnYou.later.length === 0 ? null : member({
        key: "on:waiting:later",
        verb: "more",
        stays: true,
        name: Words.name("on:waiting:later"),
        glyph: Words.glyph("on:waiting:later"),
        hue: "muted",
        side: String(WaitingOnYou.later.length),
        note: (Words.of("on:waiting:later").said ?? {}).note ?? "",
        nested: true,
        verbs: () => WaitingOnYou.later.map(each => provider.item(each)),
        targets: () => WaitingOnYou.later.map(each => provider.item(each)),
        run: () => provider.launcher?.enter(provider.laterRow)
    })

    // Everything waiting on Joel's hands, one row however many: the pi
    // sessions that want him, each the row it is in Agents (Agents.qml's
    // made), whose Return goes to its window, then what the agents have
    // waiting on him (the launcher's shape, RSCH-20261028-2NK1E5: sessions
    // wanting him first, drawn as sessions, then the orchestrator's items;
    // Now's own pi row goes). It counts and colours only what he can do
    // now, a session wanting him among it, its note names the first of
    // those, else the first for later, and the right arrow lists the
    // sessions, what's for now and then what's for later, each in its own
    // order, the oldest want first. With
    // only later things, the row is muted and says how many. With nothing
    // waiting it's still there, muted, saying so, since Now's top never
    // changes shape (the launcher's shape, RSCH-20261028-2NK1E5, landing
    // 5). It's a place, so Return steps in, as the right arrow does, to the
    // most pressing first, or to a list saying nothing waits: the row acts
    // the same whatever waits, so Super+O, Return, Return reaches the
    // pressing thing and each press can be foreseen (VKCF3Z, 2026-10-29).
    // Not a session whose want is an action, though: that's one of its own
    // items, which a session's presence mirrors (harness #907), and which
    // is listed here itself, so its session's row would say the same thing
    // twice (2026-10-31: four items read as eight).
    readonly property var sessions: Sessions.wanting
    readonly property int waitsNow: sessions.length + WaitingOnYou.now.length
    readonly property string first: sessions.length > 0
        ? `${sessions[0].title} ${Sessions.wanted(sessions[0].presence.wants)}`
        : WaitingOnYou.foremost ? WaitingOnYou.titled(WaitingOnYou.foremost) : ""
    // What's inside, made as it's stepped into.
    function waitingRows() {
        const agents = source("agents");
        return (agents ? sessions.map(session => agents.made(session)) : [])
            .concat(WaitingOnYou.now.map(each => provider.item(each)),
                laterRow === null ? [] : [laterRow]);
    }
    readonly property var waiting: member({
        // Not `on:waiting`, which is the missed notices' row, by its mark
        // (pieces/Waiting.qml), so the two are never taken for one.
        key: "on:waitingOnYou",
        verb: "more",
        stays: true,
        // What its list says when it holds nothing (Launcher.qml).
        empty: Words.of("on:waiting").empty,
        name: Words.name("on:waiting"),
        glyph: Theme.glyph.waitsOnJoel,
        hue: provider.waitsNow > 0 ? "strong" : "muted",
        side: provider.waitsNow > 0 ? String(provider.waitsNow)
            : WaitingOnYou.later.length > 0
                ? waitingSaid("later", { count: WaitingOnYou.later.length })
                : waitingSaid("none", {}),
        note: first,
        nested: true,
        verbs: () => provider.waitingRows(),
        targets: () => provider.waitingRows(),
        run: () => provider.launcher?.enter(provider.waiting)
    })
    // What home's Now leads with: the first thing waiting, after its row's
    // own name, or nothing.
    readonly property string foremost: first === "" ? ""
        : `${Words.name("on:waiting")}: ${first}`

    // The launcher, as a piece's actions are given it, so one that opens a
    // collection opens it here.
    readonly property var shell: ({ launcher: provider.launcher })

    // A thing a place of the launcher's own lists, such as a Bluetooth
    // device running low, is that place's own row, by its mark: drawn and
    // stepped into as it is there, never a second way (VKCF3Z, 2026-10-29).
    // Null for any other, or while the place doesn't list it.
    function own(fact) {
        const place = fact.piece.place ? source(fact.piece.place) : null;
        return place?.all.find(row => row.key === fact.entry.mark) ?? null;
    }

    function made(fact) {
        const piece = fact.piece;
        // Every act a piece has, where it keeps more than its notice can
        // offer (pieces/Pi.qml's every).
        const actions = (piece.every ?? piece.actions).map(action => thing({
            // Where it goes, too, since acts of one name, Go To, can
            // differ by where (pieces/Pi.qml).
            key: [`verb:on:${fact.entry.mark}`, action.name].concat(action.side ? [action.side] : [])
                .join(":"),
            name: action.name,
            side: action.side ?? "",
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

    // What's typed inside Now narrows its rows by name, as every place's
    // search does: with none, typing "time" found nothing, Time a row
    // there. A moment's row placed as an answer is made ready to match
    // here, as the launcher does for such rows.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            const prepared = row.prepared ?? Fuzzy.prepare(row.name);
            if ((prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(prepared, words, false);
            if (m.score >= 0)
                found.push({ item: prepared, score: m.score, grade: m.grade, row: row });
        }
        const kept = Fuzzy.keep(found);
        return { rows: kept.map(f => f.row), grade: kept.length > 0 ? kept[0].grade : 0 };
    }

    function everything() {
        return all.slice();
    }

    // The key of the row above, Now's top, that already stands for what's
    // on, or "": never a second row beside it (Joel, 2026-10-28: "seeing
    // multiples of the same thing gives me executive dysfunction"). The
    // sessions wanting Joel are in Waiting on You. The meeting coming
    // (Starting.qml) is the calendar's next one but for the few minutes one
    // runs on past its start, and is then the moment's meeting row.
    function above(fact) {
        if (fact.entry.mark === "agent")
            return waiting.key;
        if (fact.entry.mark === "meeting" && meeting !== null
            && meeting.key === `calendar:meeting:${fact.piece.meeting?.id ?? ""}`)
            return meeting.key;
        return "";
    }

    // This moment's rows and Waiting on You are always there, so what's
    // counted is what's on beside them, and what waits only when it does.
    function counted() {
        const waits = WaitingOnYou.foremost !== null || sessions.length > 0;
        const on = all.length - moment.length - (waits ? 0 : 1);
        return on === 0 ? "nothing" : String(on);
    }
}
