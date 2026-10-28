// What's on, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed in Now's groups, what wants Joel first,
// then what reaches other people, what's playing and the modes left on, and
// by name within each (Now.js), so what can need him leads and one keeps
// its place while its group stands; Today is always last. What each asks
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
    things: waiting === null ? [] : [waiting]

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
        // row keeps its place while its group stands (Now.js); Today, its
        // own row, always last.
        const rows = lit.map(fact => ({
            group: fact.piece.group,
            name: fact.piece.title,
            row: made(fact)
        }));
        if (waiting !== null)
            rows.push({ group: "wants", name: waiting.name, row: waiting });
        return Now.ordered(rows).map(each => each.row).concat([today]);
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
    // window it's done in, or, with none open, copies it, so it's never a
    // key that does nothing. One he can do now is strong and says its
    // minutes; one for later is muted and says when, in its owner's words.
    // The right arrow steps into it: Return's act first, then Done and Not
    // Now, which stay open as the item leaves its place, then what there is
    // to know of it.
    function item(each) {
        const now = each.ready && !each.putOff;
        const where = whereOf(each);
        const does = where !== ""
            ? act(each, "go", {
                name: Words.name("verb:on:waiting:go").replace("{where}", where),
                later: true,
                run: () => WaitingOnYou.go(each)
            })
            // The clipboard takes text only while the launcher holds the
            // keys, so a copy is made before it closes.
            : act(each, "copy", {
                now: true,
                run: () => {
                    Quickshell.clipboardText = WaitingOnYou.titled(each);
                }
            });
        const inside = () => [does,
            act(each, "done", { stays: true, run: () => WaitingOnYou.settle(each, "done") }),
            act(each, "notNow", { stays: true, run: () => WaitingOnYou.settle(each, "notNow") })
        ].concat(provider.facts(each));
        const row = member({
            key: `on:waiting:${WaitingOnYou.identity(each)}`,
            verb: does.verb,
            now: does.now ?? false,
            later: does.later ?? false,
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: now ? "strong" : "muted",
            side: !now && typeof each.when === "string" ? each.when
                : typeof each.minutes === "number"
                ? waitingSaid("minutes", { minutes: each.minutes }) : "",
            note: typeof each.why === "string" ? each.why : "",
            verbs: inside,
            run: () => does.run()
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
        now: lead.now,
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
        run: () => provider.lead.run()
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
            // One that changes how things stand, such as Let it sleep,
            // keeps the launcher, which shows it changed (Piece.qml).
            stays: action.stays ?? false,
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
            glyph: Theme.glyph[piece.markGlyph || fact.entry.mark],
            hue: Marks.tone(piece),
            plain: actions.length === 0,
            stays: actions.length > 0 && actions[0].stays,
            // Return's act leads the right arrow, then the rest, so every
            // act is there.
            verbs: actions.length > 1 ? () => actions : null,
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
