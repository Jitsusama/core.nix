// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed by name, as every list in the launcher
// is, so one keeps its place however many are on; what each asks of Joel
// shows in its glyph's colour. Return does a piece's first action, and the
// right arrow steps into the rest.
//
// It holds nothing of its own: the pieces are the margin's (Facts.qml), so
// a glyph and its row come and go together, and it adds no row to a search.
// It's home's first row, Now, saying what's in it by name.
import qs
import QtQuick
import "../Marks.js" as Marks

Provider {
    id: provider

    what: "on"

    // Named Now, as the vocabulary says (Words.qml), under the calendar
    // clock.
    collection: Words.collection("on", { searched: false })
    // What waits on Joel is found by its words too, since Now is its one
    // home: the rest of what's on each has a place of its own to be found
    // in, which is why the collection adds no row to a search.
    things: waiting === null ? [] : [waiting]

    // The kinds of piece it lists: what wants Joel, what's on air, what
    // was left on, and Today, its own row, always last.
    readonly property var groups: ["wants", "onAir", "leftOn", "today"]
    // What the thing is where the launcher has a row of its own for it, by
    // its mark, so one found in both is listed once (Rank.identity).
    readonly property var same: ({
        awake: "session:awake",
        quiet: "session:quiet",
        recording: "session:recording"
    })

    readonly property var all: {
        const lit = Facts.all.filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active);
        const rows = [];
        // By name, as every list the launcher holds is, so each keeps one
        // place however many are on (RSCH-20261025-U7566F); what each asks
        // of Joel is its glyph's colour, not its place.
        for (const kind of groups)
            lit.filter(fact => fact.piece.kind === kind).forEach(fact => rows.push(made(fact)));
        if (waiting !== null)
            rows.push(waiting);
        return rows.sort((a, b) => a.name.localeCompare(b.name, undefined,
            { numeric: true, sensitivity: "base" })).concat([today]);
    }
    onAllChanged: changed()

    // Today, Now's one row of its own, its kind "today" and always its
    // last, after what's on: the date at the right, and below it the
    // weather here and the next meeting today.
    // Return joins the meeting, or steps into it where it can't be joined
    // yet, as its own row does (the calendar provider's `next`); with no
    // meeting it steps in, to the date, the weather and the meeting, each
    // the row its own place gives. A machine without the calendar, or a
    // day with no meeting left, has no meeting half, and says nothing of
    // it.
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
        const said = weatherSource?.answer(Words.answers("weather")[0]) ?? null;
        // Still asking, failed, or not knowing where here is, it keeps what
        // it last knew rather than blinking out.
        if (said && (said.rows ?? []).length > 0)
            weather = said;
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.reweather();
        }
    }

    Connections {
        target: provider.weatherSource

        function onAnswered() {
            provider.reweather();
        }
    }

    // The day, which changes only at midnight, when it's read again.
    property date day: new Date()

    Timer {
        interval: provider.untilMidnight(provider.day)
        running: true
        onTriggered: provider.day = new Date()
    }

    function untilMidnight(from) {
        const next = new Date(from.getFullYear(), from.getMonth(), from.getDate() + 1);
        // A second past it, so the day read is surely the new one.
        return next.getTime() - from.getTime() + 1000;
    }

    // What the note says of the meeting: its name and when, "Standup in
    // 12 min".
    function meetingSaid(row) {
        return `${row.name} ${row.side}`;
    }

    readonly property var today: {
        const date = Qt.formatDate(day, "dddd d MMMM");
        const said = [weather?.headline ?? "", meeting ? meetingSaid(meeting) : ""]
            .filter(part => part !== "");
        const inside = () => {
            const rows = [copyRow({
                key: "on:today:date",
                name: Words.name("today"),
                side: Qt.formatDate(day, "dddd d MMMM yyyy"),
                glyph: Theme.glyph.date
            })];
            if (weather)
                rows.push(holderRow({
                    key: "on:today:weather",
                    name: Words.name("weather"),
                    side: weather.headline,
                    note: weather.note ?? "",
                    hue: "voice",
                    // The sky's own glyph, as the weather's Now row wears it.
                    glyph: weather.rows[0].glyph
                }, () => weather.rows));
            if (meeting)
                rows.push(meeting);
            return rows;
        };
        const row = holderRow({
            key: "on:today",
            name: Words.name("on:today"),
            side: date,
            note: said.join(" \u00b7 "),
            hue: "voice",
            glyph: Theme.glyph.date
        }, inside);
        // With a meeting, Return is the meeting's own: joining it, or
        // stepping into it where it can't be joined yet.
        if (meeting)
            Object.assign(row, {
                verb: meeting.verb,
                stays: meeting.stays ?? false,
                later: meeting.later ?? false,
                run: () => meeting.run()
            });
        return row;
    }

    function waitingSaid(id, fill) {
        let words = (Words.of("on:waiting").said ?? {})[id] ?? id;
        for (const key of Object.keys(fill))
            words = words.replace(`{${key}}`, fill[key]);
        return words;
    }

    // One thing waiting on Joel, as the agents keep them (Steward.qml):
    // its title, how long it takes, and why it waits; Return goes to the
    // window it's done in. One he can do now says so by its glyph's
    // colour, never by its place.
    function item(each) {
        return thing({
            key: `on:waiting:${Steward.identity(each)}`,
            verb: Words.verb("on:waiting"),
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: each.ready ? "strong" : "muted",
            side: typeof each.minutes === "number"
                ? waitingSaid("minutes", { minutes: each.minutes }) : "",
            note: typeof each.why === "string" ? each.why : "",
            run: () => Steward.go(each)
        });
    }

    // What the agents have waiting on Joel, one row however many: its note
    // names the first he can do now, else the first in the agents' own
    // order, which Return goes to, and the right arrow lists them all, by
    // name. With nothing waiting, no row.
    readonly property string first: Steward.foremost ? Steward.titled(Steward.foremost) : ""
    readonly property var waiting: Steward.items.length === 0 ? null : member({
        key: "on:waiting",
        verb: Words.verb("on:waiting"),
        name: Words.name("on:waiting"),
        glyph: Theme.glyph.waitsOnJoel,
        hue: Steward.items.some(each => each.ready) ? "strong" : "muted",
        side: String(Steward.items.length),
        note: first,
        verbs: () => Steward.items.map(each => provider.item(each))
            .sort((a, b) => a.name.localeCompare(b.name, undefined,
                { numeric: true, sensitivity: "base" })),
        run: () => Steward.go(Steward.foremost)
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
        const actions = piece.actions.map(action => thing({
            key: `verb:on:${fact.entry.mark}:${action.name}`,
            name: action.name,
            glyph: action.glyph,
            hue: "action",
            chord: action.chord ?? [],
            run: () => action.run(provider.shell)
        }));
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
            glyph: Theme.glyph[fact.entry.mark],
            hue: Marks.tone(piece),
            plain: actions.length === 0,
            verbs: actions.length > 1 ? () => actions.slice(1) : null,
            run: () => {
                if (actions.length > 0)
                    actions[0].run();
            }
        });
    }

    function everything() {
        return all.slice();
    }

    // Today is always there, so what's counted is what's on beside it.
    function counted() {
        return all.length === 1 ? "nothing" : String(all.length - 1);
    }
}
