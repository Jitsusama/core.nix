// Now, a collection, and Super+O's: a row for each glyph in the
// margin (Margin.qml), saying in words what the glyph only pictures. Each
// row is the glyph in the colour the margin gives it, the piece's name, and
// how it stands at the right, with what more there is to know in the footer
// while it's picked. They're listed in Now's groups, what wants Joel first,
// then what reaches other people, what's playing and the modes left on, and
// by name within each (NowGroups.js), so what can need him leads and one keeps
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
import Quickshell.Io
import QtQuick
import Qt.labs.folderlistmodel
import "../Fuzzy.js" as Fuzzy
import "../Hearing.js" as Hearing
import "../Marks.js" as Marks
import "../When.js" as When
import "Ago.js" as Ago
import "ForYou.js" as ForYou
import "ForYouFilter.js" as Filter
import "Lately.js" as Lately
import "NowGroups.js" as Groups
import "Showing.js" as Showing
import "Order.js" as Order
import "Moment.js" as Moment
import "NeedsYou.js" as NeedsYou
import "Here.js" as Here
import "PlaceRow.js" as PlaceRow
import "../pieces/Holding.js" as Holding

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

    // Made only while Now's rows are wanted (Provider.qml's whileWanted),
    // as are its location's, For You's own, Later's and History's, since
    // a piece moving, the minute turning and anything For You counts each
    // made them again: 199 in four minutes with the launcher shut on
    // Joel's shell (YQZGPC, 2026-11-10). The tray's count is ForYouHolds',
    // and home's row says what homeItems reads, so neither waits on them.
    readonly property var all: whileWanted("all", () => {
        const lit = Facts.all.filter(fact => fact.entry.mark !== null && fact.piece
            && fact.piece.present && fact.piece.active && above(fact) === "");
        // What waits on Joel, then this moment's rows, which are always
        // there, so the top never moves; then the rest by group, what
        // wants Joel first, and by name within each, so a row keeps its
        // place while its group stands (NowGroups.js).
        keepingBegun(piecesKept);
        for (const mark of Object.keys(piecesNow))
            delete piecesNow[mark];
        for (const fact of lit)
            piecesNow[fact.entry.mark] = fact.piece;
        const rows = lit.map(fact => ({
            group: fact.piece.group,
            name: fact.piece.title,
            row: own(fact) ?? keptOr(piecesKept, fact.entry.mark, pieceSays(fact),
                () => made(fact))
        }));
        keptOnly(piecesKept);
        // What sessions landed that says what to try waits in For You, so
        // it's counted once there and never lifted here too (VKCF3Z,
        // 2026-11-10).
        return [waiting].concat(moment, Groups.ordered(rows).map(each => each.row));
    })
    // Said only when what the rows show has changed (Provider's
    // changedTo), since `all` is made again whenever any fact moves, and
    // each change said makes the launcher refresh (Joel, 2026-11-08: "weird
    // lag can happen").
    onAllChanged: changedTo(all)

    // Each piece's row, with its acts, kept while what it shows reads the
    // same: `all` is made again whenever any fact moves, and made every
    // lit piece's rows anew each time, about 12 a change on Joel's shell
    // (2026-11-10). What its row says is read from the piece as text
    // (pieceSays); what it does and draws is asked of the piece as it's
    // done, the piece by its mark as `all` last read it (piecesNow), since
    // a thing a piece stands for is made anew as its piece moves
    // (Facts.js). Written into, never reassigned, so it notifies nothing.
    readonly property var piecesKept: keeper()
    readonly property var piecesNow: ({})
    function pieceSays(fact) {
        const piece = fact.piece;
        const acts = list => (list ?? []).map(action => [action.name, action.side ?? "",
            action.glyph ?? "", action.chord ?? [], action.stays ?? false, action.verb ?? ""]);
        return JSON.stringify([provider.same[fact.entry.mark] ?? "", piece.title, piece.value,
            piece.valueItems ?? [], piece.counts ?? null, parted(piece), piece.heldNames ?? null,
            piece.widest ?? null, piece.context ?? "", piece.markGlyph ?? "", Marks.tone(piece),
            acts(piece.actions), acts(piece.every), typeof piece.rows, typeof piece.drawer,
            piece.drawnBy ?? null]);
    }

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
                provider.openedAt = Date.now();
                provider.reweather();
                // Placing the time and the date makes Time's and Today's
                // rows, so an opening into a place that shows none of Now's
                // leaves them stale until Now is wanted (H6EDLC, 2026-11-11),
                // and one onto Now's rows places them only once a minute
                // has turned since they were placed: each opening into For
                // You made all 17 again with nothing changed (2026-11-11).
                // Asleep as a kept opening fills in, it wakes a frame or two
                // on and is wanted then (onWantedChanged), within the minute
                // it was placed in, so it's kept: a chord into Now kept made
                // all 17 again as it woke (2026-11-11).
                if (provider.wanted)
                    provider.retimeIfStale();
                else if (!provider.asleep)
                    provider.timeStale = true;
            } else {
                provider.openedAt = Number.POSITIVE_INFINITY;
            }
        }
    }

    // When the launcher last opened, so a notice read since stays where
    // Joel read it until it closes (latelyNotices).
    property real openedAt: Number.POSITIVE_INFINITY

    // The time and the date as the launcher places them, read rather than
    // bound: the launcher's answers are made from every provider's rows,
    // Now's among them, so a binding here would read itself back (a loop
    // Quickshell warns of as it starts). Read as the clock turns a minute,
    // as the launcher opens onto Now's rows or comes to them, and once the
    // shell has started.
    property var timeRow: null
    property var dateRow: null

    function retime() {
        const placed = id => provider.launcher?.resolved?.(id) ?? null;
        timeRow = placed("time");
        dateRow = placed("today");
        timedMinute = provider.minuteNow();
        timeStale = false;
    }

    // The minute the time and the date were last placed in, so an opening
    // within it keeps them, and the minute it is now.
    property real timedMinute: -1

    function minuteNow() {
        return Math.floor(Date.now() / 60000);
    }

    // Whether a minute turned, or the launcher opened, while Now's rows
    // weren't wanted, so it's read as they are again. Placing the time and
    // the date makes Time's and Today's rows, so an opening onto Now's
    // rows places them once, and only if they're stale or a minute has
    // turned since.
    property bool timeStale: false

    function retimeIfStale() {
        if (timeStale || timedMinute !== minuteNow())
            retime();
    }

    Connections {
        target: clock

        function onDateChanged() {
            if (provider.wanted)
                Qt.callLater(provider.retime);
            else
                provider.timeStale = true;
        }
    }

    Component.onCompleted: Qt.callLater(retime)
    onLauncherChanged: Qt.callLater(retime)
    onWantedChanged: {
        if (wanted)
            Qt.callLater(retimeIfStale);
    }

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

    // What a piece's drawer says can move each second, as a song's
    // position does, so its drawer is read again each second, but only
    // while the launcher is open, where it can be seen: a clock running
    // all day would wake the shell each second for nothing (Y2YBTS,
    // 2026-11-08). Whether it runs is said for the tests.
    readonly property bool drawing: drawnClock.enabled

    SystemClock {
        id: drawnClock
        enabled: provider.launcher?.open ?? false
        precision: SystemClock.Seconds
    }

    // Where the laptop is, as Location says it, the one place the weather
    // and everything else that depends on it reads here from (Joel,
    // 2026-10-28: "a location area in now ... tied to what weather and
    // any other geo-conscious options pull from"): its town, else its
    // position, and how sure it is, saying so where the place keeps another
    // zone than the clock. Inside, in the design's order (VKCF3Z's design,
    // "Location. Inside: Town, How It Knows, Set It by Hand, Find It Again,
    // Copy"): its town and its country, how it knows and when it was
    // located, setting it by hand and finding it again, and Copy, of its
    // position, last. Its plate says when, how near, where on a map, its
    // country and its zone (Here.js; VKCF3Z, 2026-11-08). Found by typing
    // its name too, since Now is its home. Put It Back after the clock
    // followed it is Time's (Clocks.qml), since the zone is the clock's.
    //
    // GeoClue's altitude, speed and heading are on the plate where its
    // source gives them, as GPS does; Wi-Fi and a network's address give none,
    // and the plate's body names what's missing and the source that
    // couldn't give it, as design.md has what's left out named on screen
    // with its reason. The source is the plate's eyebrow (VKCF3Z,
    // 2026-11-09).
    readonly property var location: whileWanted("location", () => {
        const here = Location.here;
        const position = here ? `${here.latitude.toFixed(2)}, ${here.longitude.toFixed(2)}` : "";
        // A place Joel set himself says so, since GeoClue no longer moves it.
        const byHand = here?.hand === true;
        const said = hereSaid;
        const within = !here ? "" : byHand ? said.hand : Here.within(here.accuracy, said, hereTold);
        const side = here ? here.place ?? position : Location.read ? said.unknown : "\u2026";
        const differs = Location.zoneDiffers;
        // Said plainly where it's unknown, never guessed, as a run that
        // gives way by whole items: the zone it keeps, then that the
        // clock isn't following it, then why.
        const elsewhere = !differs ? "" : [
            said.elsewhere.replace("{zone}", here.zone),
            Here.unfollowed(here, said, hereTold)
        ].filter(item => item !== "").join(" \u00b7 ");
        // How near it is, its source being the plate's eyebrow, and the
        // machinery that found it named nowhere (VKCF3Z, 2026-11-09).
        const unfound = Location.read ? said.notFound : "";
        const placed = differs ? elsewhere : within;
        const note = here ? placed : unfound;
        const now = clock.date.getTime();
        // Its rows inside, as whereabouts serves them (`whereabouts`).
        const inside = () => whereabouts.inside("row:on:location");
        // Its plate: how it was found, then its kind, as an eyebrow says
        // where then what, and what of it the side doesn't say; with
        // nothing found, none. It reads here as it's drawn, since the
        // launcher keeps a row that reads the same, and a source or GPS's
        // measures can change while the town and its note don't.
        const plate = !here ? null : () => plateOf(Location.here ?? here, now, differs);
        // Kept while it says the same of the same place in the same minute,
        // since Now wanted again remade it at every opening into For You
        // with nothing changed (2026-11-11).
        return keptAlike(ownKept, "location", [here, side, note, differs, now],
            () => thingRow(Object.assign({
                key: "on:location",
                name: Words.name("location"),
                side: side,
                note: note,
                hue: differs ? "strong" : "voice",
                glyph: Theme.glyph.location
            }, plate ? { thing: true, drawer: plate } : {}), inside));
    })

    // Location's own rows, inside its row, as whereabouts serves them
    // worded and placed (whereabouts/src/rows.rs), held over a connection
    // of their own whether or not the launcher's open, so stepping in finds
    // them held. Each is made as every place's are, a figure copying what
    // it says and an act asked back on the same socket (ReadyRows.qml).
    ReadyRows {
        id: whereabouts
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/location.sock`
        place: "location"
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => whereabouts.thing(row, key, provider.locationsOwn(row))
    }
    // What whereabouts serves, for a reader told of each change it hears.
    readonly property alias whereaboutsRows: whereabouts

    // What a row of Location's own takes of the launcher's past what's
    // served: Set It by Hand asks Joel for a town first, in the words its
    // key has, and says how it went at the foot; Copy reads as an act
    // rather than a figure; and when it was found, a time the shell words,
    // copies it as said.
    function locationsOwn(row) {
        switch (row.thing) {
        case "verb:location:set":
            return {
                ask: Words.of(row.thing).ask ?? "",
                chip: Words.name("location"),
                state: () => Location.told,
                given: words => Location.setByHand(words),
                stays: false,
                run: undefined
            };
        case "verb:location:copy":
            return { figure: false, plain: false };
        case "location:located": {
            const said = () => whereabouts.sideOf(row);
            return {
                verb: "copy",
                figure: true,
                plain: true,
                now: true,
                copies: said,
                run: () => {
                    if (said() !== "")
                        Quickshell.clipboardText = said();
                }
            };
        }
        }
        return {};
    }

    // Location's plate for here, as `location` says it.
    function plateOf(here, now, differs) {
        return {
            eyebrow: Here.eyebrow(here, hereSaid),
            eyebrowNames: true,
            // Its town, the place's first part, as the plate names the
            // thing it's about.
            title: (here.place ?? "").split(",")[0] || Words.name("location"),
            facts: Here.besideNote(Here.facts(here, now, hereSaid, hereTold), !differs)
                .map(fact => ({
                    label: Words.name(`location:${fact.id}`),
                    value: fact.value,
                    identifies: fact.identifies === true
                })),
            // What its source couldn't say of it.
            body: Here.leftOut(here, hereSaid, Words.of("location").tells ?? {})
        };
    }

    // How Location says a time, a day and how long ago, as the launcher
    // says them everywhere.
    readonly property var hereSaid: Words.of("location").said ?? {}
    readonly property var hereTold: ({
        ago: (at, now) => Ago.ago(at, now),
        day: (at, now) => When.day(at, now, {
            yesterday: provider.hereSaid.yesterday ?? "",
            date: (date, format) => Qt.formatDate(date, format)
        }),
        clock: at => When.clock(at),
        number: (value, decimals) => Qt.locale().toString(value, "f", decimals)
    })

    // Made again as the clock turns a minute, so the time says it.
    readonly property var moment: {
        const rows = [timeRow, dateRow, location];
        if (weather)
            rows.push(weather);
        if (meeting)
            rows.push(meeting);
        return rows.filter(row => row !== null);
    }

    // Home's Now as a plate beside the card (Launcher.qml's whole): the
    // date as its eyebrow, the time large with its seconds (Drawer.qml
    // keeps them, counting), and the facts behind the moment in two
    // groups, as the card's rows fall in groups: the day's weather, the
    // next meeting and where here is, then how the clock is kept and the
    // week. Each fact's figure leads, and
    // what qualifies it follows, quieter (VKCF3Z's look 3, for Joel's
    // "for time i want to be able to dig into ntp server details", 29
    // October). A fact nothing can say yet is left out rather than guessed.
    // Named for the place it stands beside, so another source's own plate,
    // as Home Network's for one of its rows, is never taken for it.
    function placePlate() {
        const said = key => String((Words.of("on:plate").said ?? {})[key] ?? key);
        // Each fact named as its thing is: the weather and the week by
        // their own, the rest by the plate's.
        const fact = (thing, value, qualifier) => value === "" ? null
            : { label: Words.name(thing), value: value, qualifier: qualifier ?? "" };
        const now = new Date();
        // The weather as its answer heads it, "4\u00b0 \u00b7 rain to 23:00".
        const [degrees, ...weatherRest] = String(weather?.side ?? "").split(" \u00b7 ");
        // The next meeting by its name, then when it starts.
        const starts = String(meeting?.side ?? "").match(/\d{1,2}:\d\d/);
        const here = Location.here;
        const how = !here ? "" : here.hand === true ? said("byHand")
            : Here.within(here.accuracy, hereSaid, hereTold);
        // How the clock is kept, in Time's word for it (Clocks.qml), and
        // how far off it was at the last poll, signed, as Time says it.
        const time = source("time");
        const kept = time?.kept ?? null;
        const offset = kept?.exchange ? time.millis(kept.exchange.offset, true) : "";
        const step = !kept ? said("unkept") : kept.synchronised ? said("synchronised")
            : said("unsynchronised");
        return {
            now: true,
            eyebrow: Moment.eyebrow(now, now),
            title: Words.name("on"),
            // How the seconds are said after the time, "and 12 s".
            seconds: said("seconds"),
            // The plate says nothing of what waits: home's row says it at
            // its right (homeItems), and no drawer repeats its row; calm,
            // it says nothing either, as the row does, so it holds only
            // the moment (VKCF3Z, 2026-11-09).
            groups: [
                [fact("weather", degrees ?? "", weatherRest.join(" \u00b7 ")),
                    fact("on:plate:next", meeting?.name ?? "", starts ? starts[0] : ""),
                    fact("on:plate:here", here ? here.place ?? "" : "", how)],
                [fact("on:plate:clock", step, offset),
                    fact("on:plate:week", String(Moment.isoWeek(now).week),
                        said("day").replace("{day}", String(Moment.ordinal(now))))]
            ].map(group => group.filter(one => one !== null))
        };
    }

    function waitingSaid(id, fill) {
        return filledIn((Words.of("on:waiting").said ?? {})[id] ?? id, fill);
    }

    // The line Deal With It fills a session's prompt with: a sentence of
    // its own, so it's told rather than said.
    function readyToDeal(title) {
        return filledIn((Words.of("on:waiting").tells ?? {})["ready-to-deal"] ?? title,
            { title: title });
    }

    function filledIn(words, fill) {
        for (const key of Object.keys(fill))
            words = words.replace(`{${key}}`, fill[key]);
        return words;
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

    // What an item's note tells beyond its title, worked out apart from the
    // view (NeedsYou.js) from what this says in words: its times as the
    // clock says them, the machine it's done on, whether anything will see
    // it done, and its side, so its note never says the side again.
    // In For You, its side is the strongest reason it's there (`reason`).
    function toldOf(each, reason) {
        return {
            watched: WaitingOnYou.watched(each),
            side: (reason ?? "") !== "" ? reason : sideOf(each),
            ready: each.ready === true && !each.putOff,
            now: Date.now()
        };
    }

    // Where and by when it's done, each where its owner said, that nobody's
    // watching it when only Joel can say it's done and the session that
    // raised it has gone (VKCF3Z, 2026-10-30), led by its owner's words for
    // when and ended by how long it's waited (VKCF3Z, 2026-11-07).
    function noteOf(each, reason) {
        return NeedsYou.note(each, Words.of("on:waiting").said ?? {}, toldOf(each, reason));
    }

    // Its drawer: where it's from and that it's a thing to do, its title,
    // then its facts, what it weighs and why, since when it has waited, how
    // it'll be known done, its session, and who it is last; then
    // its steps, numbered, and why only Joel can do it (VKCF3Z, 2026-10-31
    // and 2026-11-08), each only when it says it. How it's known done is a
    // fact now, so the words don't say it again.
    // `reason` is the weighed row's side, "" for one not weighed, such as
    // Later's, whose side says what it waits for.
    function drawerOf(each, reason) {
        const steps = (Array.isArray(each.steps) ? each.steps : [])
            .filter(step => typeof step === "string" && step !== "")
            .map((step, at) => `${at + 1}. ${step}`);
        const why = typeof each.why === "string" ? each.why : "";
        const now = Date.now();
        const say = forYouSaid(now);
        const session = WaitingOnYou.session(each);
        const quest = WaitingOnYou.questOf(each) ?? "";
        const since = ForYou.when(each.raisedAt);
        const checked = ForYou.when(each.checkedAt);
        const owner = ForYou.figures(each, {
            now,
            sideSaysDue: (reason ?? "") !== "" && itemThing(each).blocks === 0
                && ForYou.when(each.deadline) !== null,
            window: WaitingOnYou.windowOf(each)?.title ?? ""
        }, say);
        // An identifier is set in the Name face; a check's command stands
        // under its name, wrapping at its own punctuation (Drawer.qml).
        const identifier = (id, value, under) => Object.assign(fact(id, value),
            { identifies: true }, under ? { words: true } : {});
        const facts = [
            fact("weight", ForYou.weightSaid(itemThing(each), now, say)),
            fact("due", owner.due),
            fact("set-for", owner.setFor),
            fact("takes", typeof each.minutes === "number"
                ? waitingSaid("takes", { minutes: each.minutes }) : ""),
            fact("since", since === null ? "" : say.clock(since)),
            // When the session that raised it last found it still true, so
            // Joel can tell an ask that's gone stale (Joel, 2026-11-08: "i
            // don't know if it is still relevant").
            fact("checked", checked === null ? "" : say.clock(checked)),
            // A sentence, so it's set in Words, under its name.
            Object.assign(fact("clears", knownSaid(each)), { words: true }),
            fact("machine", owner.machine),
            fact("where", owner.where),
            fact("session", ForYou.sessionSaid(session?.title ?? "",
                WaitingOnYou.owners[each.quest]?.status)),
            identifier("address", owner.address, false),
            identifier("check", owner.check, true),
            Object.assign(fact("item", WaitingOnYou.identity(each)), { identifies: true })
        ].filter(one => one.value !== "");
        return {
            // Its quest by its title (VKCF3Z, 2026-11-08); its owner's
            // address is a fact of its own.
            eyebrow: waitingSaid("from", { from: quest || session?.title || "", kind: say.todo }),
            eyebrowNames: true,
            title: each.title,
            facts: facts,
            body: steps.concat(why === "" ? [] : ["", why]).join("\n").trim()
        };
    }

    // Whether the session or quest that raised an item is gone, as its
    // record says (harness: `owner.state`), or "" while it's there.
    function ownerSaid(each) {
        const state = each.owner?.state;
        return state === "ended" || state === "concluded" ? waitingSaid(state, {}) : "";
    }

    // Takes Joel to the session an item is from, ready to deal with it: its
    // window comes forward with its prompt filled in, never sent, naming
    // the item and asking the session to check it still holds before it
    // asks him anything (Joel, 2026-11-08: "it should give me a prompt that
    // gets filled into it stating i'm reayd to deal with it if it is still
    // valid"). The harness fills a prompt only while it's empty.
    function dealWith(each) {
        const session = WaitingOnYou.session(each);
        if (session)
            Sessions.steer(session.address, {
                act: "fill",
                text: readyToDeal(each.title)
            });
        WaitingOnYou.go(each);
    }

    // A fact of For You's drawers, named in the vocabulary.
    function fact(id, value) {
        return { label: Words.name(`on:waiting:${id}`), value: String(value ?? "") };
    }

    // The words For You's rows and drawers are said in, with the clock and
    // how long ago as the launcher says them (Ago.js), at `now`.
    function forYouSaid(now) {
        const said = Words.of("on:waiting").said ?? {};
        const say = Object.assign({}, said, {
            ago: at => Ago.ago(at, now),
            day: (at, today) => When.day(at, today, {
                yesterday: said.yesterday,
                tomorrow: said.tomorrow,
                date: (date, format) => Qt.formatDate(date, format)
            }),
            time: at => When.clock(at)
        });
        // Every moment For You says, one way (ForYou.js's moment): When's
        // rule, with When's day and clock handed in, which its UTC case gives
        // in their place.
        say.clock = at => ForYou.moment(at, now, say);
        return say;
    }

    // An item as ForYou.js weighs it: a thing to do, ready while Joel can do
    // it and hasn't put it off, due by its deadline, waiting since it was
    // raised, and blocking each session whose want is it.
    function itemThing(each) {
        return {
            kind: "todo",
            blocks: ForYou.blocked(each, provider.wantingHeard),
            due: each.deadline ?? null,
            ready: each.ready === true && !each.putOff,
            since: each.raisedAt ?? null
        };
    }

    // How an item will be known done, in words (Waiting.doneBy): by its
    // check or signal, in the phrases what it waits for is said in, or by
    // Joel alone; "" when it doesn't say.
    function knownSaid(each) {
        const done = WaitingOnYou.doneBy(each);
        if (done === null)
            return "";
        // Sentences of the drawer's own, as words.json's `tells` holds them.
        const tells = Words.of("on:waiting").tells ?? {};
        if (done.person)
            return tells.person ?? "";
        const how = waitingSaid(done.check ? "check" : done.signal, { value: done.value ?? "" });
        return how === "" ? "" : (tells.known ?? "").replace("{how}", how);
    }

    // What its side says: for one not ready, what it waits for, else how
    // long it takes.
    function sideOf(each) {
        const now = each.ready && !each.putOff;
        return !now && waitsSaid(each) !== "" ? waitsSaid(each)
            : typeof each.minutes === "number"
            ? waitingSaid("minutes", { minutes: each.minutes }) : "";
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
    // In For You's weighed list (`weighed`, the time it's weighed at), its
    // side says the strongest reason it's there instead (ForYou.js's reason),
    // and its row is kept while what it says reads the same (itemsKept).
    function item(each, weighed) {
        const now = each.ready && !each.putOff;
        const reason = weighed === undefined ? ""
            : ForYou.reason(itemThing(each), weighed, forYouSaid(weighed));
        // An ended session or concluded quest is a state, so it leads the
        // side, said once (VKCF3Z, 2026-11-08).
        const side = [ownerSaid(each), reason !== "" ? reason
            : !now && waitsSaid(each) !== "" ? waitsSaid(each)
            : typeof each.minutes === "number"
            ? waitingSaid("minutes", { minutes: each.minutes }) : ""]
            .filter(part => part !== "").join(" \u00b7 ");
        const note = noteOf(each, reason);
        if (weighed === undefined)
            return itemRow(each, () => each, now, side, note, reason);
        const identity = WaitingOnYou.identity(each);
        itemsNow[identity] = each;
        return keptAlike(itemsKept, identity, [each.title, now, side, note],
            () => itemRow(each, () => provider.itemsNow[identity] ?? each, now, side, note,
                reason));
    }

    // Each to-do's row in For You, by its identity, kept while its title,
    // whether Joel can do it now, its side and its note read the same,
    // which is all its row says; what's inside is made as it's stepped
    // into. For You is asked what it holds at each refresh while it's
    // held, and the node says its to-dos anew at each revision, so each
    // was made anew, its acts too, at every change in Now (2026-11-10).
    // Each to-do as For You last read it (itemsNow) is what a kept row's
    // acts and drawer read. Plain, so keeping notifies nothing.
    readonly property var itemsKept: keeper()
    readonly property var itemsNow: ({})

    // An item's row, `current()` giving the item as it is now.
    function itemRow(each, current, now, side, note, reason) {
        // Go To, where it goes at its side, as going to anything open is
        // said everywhere, so the word never changes shape.
        const does = one => {
            const where = whereOf(one);
            return where === "" ? null : act(one, "go", {
                side: where,
                later: true,
                run: () => WaitingOnYou.go(one)
            });
        };
        // Dealing with it in its session leads, when it has one to go to.
        const deal = one => WaitingOnYou.session(one) === null ? null : act(one, "deal", {
            side: WaitingOnYou.session(one).title,
            later: true,
            run: () => provider.dealWith(one)
        });
        // Its quest, said as the act it is (VKCF3Z, 2026-11-08: acts are
        // imperatives), its title at its side; Return opens the quest's row
        // as Quests has it, which never closes the launcher.
        const quest = one => {
            const found = (source("quests")?.everything() ?? [])
                .find(row => row.key === `quest:${one.quest}`) ?? null;
            return found === null ? null : act(one, "quest", {
                side: found.name,
                stays: true,
                drawer: found.drawer,
                run: () => provider.launcher?.enter(found)
            });
        };
        // An item done in a window of its own leads with going there, its
        // app; one done in its session leads with dealing with it there.
        const inside = () => {
            const one = current();
            const own = typeof one.window === "string" && one.window !== "";
            return (own ? [does(one), deal(one)] : [deal(one), does(one)]).concat([
                act(one, "done", { stays: true, run: () => WaitingOnYou.settle(one, "done") }),
                act(one, "notNow", { stays: true,
                    run: () => WaitingOnYou.settle(one, "notNow") }),
                quest(one)
            ]).filter(row => row !== null);
        };
        const row = member({
            thing: true,
            key: `on:waiting:${WaitingOnYou.identity(each)}`,
            // What it is and where it's from, which For You filters by; not
            // `kind`, which a row says beside its side (Choice.qml).
            forYou: { kind: "do", source: "needs-you" },
            // Return steps in, to what it is, why and how, never straight
            // into a session past what's asked (Joel, 2026-11-08: "Selecting
            // one brings me often into a pi session that is so far past what
            // i'm being asked that i don't know if it is still relevant").
            verb: "more",
            stays: true,
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: now ? "strong" : "muted",
            side: side,
            note: note,
            drawer: () => provider.drawerOf(current(), reason),
            // Its verbs and targets both make it a row holding rows, which
            // Return's own act never leads (Catalogue.js's leading).
            verbs: inside,
            targets: inside,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // What can't be done yet, one row at the end of the list, so the list
    // holds only what Joel can do now; each inside says what it waits for.
    // Kept while it names the same, since the node says what's for later
    // anew at each revision (ownKept).
    readonly property var laterRow: whileWanted("later", () => WaitingOnYou.later.length === 0
        ? null : keptAlike(ownKept, "later", [WaitingOnYou.later.map(each => each.title)
            .join("\n")], () => member({
        key: "on:waiting:later",
        verb: "more",
        stays: true,
        name: Words.name("on:waiting:later"),
        glyph: Words.glyph("on:waiting:later"),
        hue: "muted",
        side: String(WaitingOnYou.later.length),
        // A row holding parts names its members, as Timers does, by whole
        // names rather than a sentence (VKCF3Z, 2026-11-08).
        note: WaitingOnYou.later.map(each => each.title).join(", "),
        nested: true,
        verbs: () => WaitingOnYou.later.map(each => provider.item(each)),
        targets: () => WaitingOnYou.later.map(each => provider.item(each)),
        run: () => provider.launcher?.enter(provider.laterRow)
    })))
    // For You's own rows, Later and For You itself, each kept while what
    // it says reads the same: each was made anew as anything it counts
    // moved, and For You's with it all of what's on (2026-11-10). Plain,
    // so keeping notifies nothing.
    readonly property var ownKept: keeper()

    // What the agents left for Joel to look at, each a file of the looks
    // folder the harness writes, ~/.local/state/pi/agentic-harness.pi/
    // looks/<id>.json: a pull request, a page, a file, a diff or a
    // terminal, with why. The folder is listed only while the launcher is
    // open and each file read then, so a closed launcher costs nothing and
    // nothing is polled (VKCF3Z, 2026-11-08); what was read last stays
    // while it's closed, so For You opens on it before the folder answers.
    readonly property string looksAt: `${Quickshell.env("XDG_STATE_HOME")
        || `${Quickshell.env("HOME")}/.local/state`}/pi/agentic-harness.pi/looks`
    // Each file's text, by its name, as it was last read.
    property var lookTexts: ({})
    // Each file's look is kept by its text, in looksParsed.by, so only a
    // file that changed is parsed again. A stopgap: KP0L6A's agents daemon
    // takes the looks over by Sat 14 Nov, and For You takes its rows from
    // it ready, with nothing to read or parse here.
    readonly property var looks: ForYou.looks(Object.keys(lookTexts)
        .map(file => ({ file: file, text: lookTexts[file] })), looksParsed)
    readonly property var looksParsed: ({})

    FolderListModel {
        id: lookFiles
        folder: provider.launcher?.open ? `file://${provider.looksAt}` : ""
        nameFilters: ["*.json"]
        showDirs: false
        showDotAndDotDot: false
        // A file gone from the folder goes from what's read, once it has
        // been listed whole.
        onStatusChanged: provider.relisted()
        onCountChanged: provider.relisted()
    }

    Instantiator {
        model: lookFiles

        FileView {
            required property string fileName
            required property string filePath

            path: filePath
            printErrors: false
            onLoaded: provider.readLook(fileName, text())
        }
    }

    // A file as it was read, as the launcher opens: every file is read
    // again at each opening, so one reading as it did changes nothing,
    // and the rest are taken together once the turn's reads are done.
    // Taken one by one, each parsed every file again and banded For You
    // afresh, 166 times in each of Joel's openings, 60 to 82% of the
    // 6.5 s his shell's thread was pegged (H6EDLC's profile, 2026-11-10).
    function readLook(file, text) {
        if (!(file in looksRead) && lookTexts[file] === text)
            return;
        looksRead[file] = text;
        Qt.callLater(looksReadDue);
    }
    readonly property var looksRead: ({})

    function looksReadDue() {
        const files = Object.keys(looksRead);
        if (files.length === 0)
            return;
        const next = Object.assign({}, lookTexts);
        for (const file of files) {
            next[file] = looksRead[file];
            delete looksRead[file];
        }
        lookTexts = next;
    }

    // A file's text as Now wrote it, taken at once, so the act that wrote
    // it is seen in its own frame; a read of it still waiting is older.
    function heardLook(file, text) {
        delete looksRead[file];
        lookTexts = Object.assign({}, lookTexts, { [file]: text });
    }

    // Only the looks folder's listing says which looks are gone: shut, the
    // model is given no folder and lists the shell's working one instead,
    // which holds none of them.
    function relisted() {
        if (lookFiles.folder.toString() !== `file://${looksAt}`
            || lookFiles.status !== FolderListModel.Ready)
            return;
        const listed = {};
        for (let at = 0; at < lookFiles.count; at += 1)
            listed[lookFiles.get(at, "fileName")] = true;
        const kept = Object.keys(lookTexts).filter(file => listed[file]);
        if (kept.length === Object.keys(lookTexts).length)
            return;
        const next = {};
        for (const file of kept)
            next[file] = lookTexts[file];
        lookTexts = next;
    }

    // A look's file written again whole, as it's done with, through a file
    // beside it renamed over it (atomicWrites), so a reader never sees half
    // of one. It's written off the shell's thread: written there, with its
    // sync to disk, it took 3 to 31 ms of Joel's act, 70 on the first
    // (2026-11-10). Each file has a writer of its own until it's written,
    // since a writer given a second write gives up the first: a second
    // write of the same file is the one meant, and replaces the first. A
    // write that fails puts the look back as it read before the act and
    // says so, since a look shown cleared that comes back at the next read
    // would say nothing of why (Y2YBTS, 2026-11-10).
    Component {
        id: lookWrite

        FileView {
            id: writer

            required property string file
            // The file's text before the act, and the text written.
            required property string was
            property string wrote: ""

            preload: false
            printErrors: false
            atomicWrites: true
            blockWrites: false
            onSaved: provider.lookWritten(file, writer)
            onSaveFailed: error => provider.lookUnwritten(file, writer, error)
        }
    }
    // Each file's writer, by its name, while its write is under way.
    readonly property var lookWriters: ({})

    function settleLook(one, state, how) {
        if (one.file === "")
            return;
        const text = ForYou.rewritten(one, state, how, Date.now());
        if (!(one.file in lookWriters))
            lookWriters[one.file] = lookWrite.createObject(provider, { file: one.file,
                was: lookTexts[one.file] ?? "", path: `${provider.looksAt}/${one.file}` });
        lookWriters[one.file].wrote = text;
        // Taken before it's written, so a write that fails, even within
        // setText, finds the text it wrote.
        heardLook(one.file, text);
        lookWriters[one.file].setText(text);
    }

    // Put back only while nothing has read or written the file since.
    function lookUnwritten(file, writer, error) {
        console.warn(`Now: ${looksAt}/${file} wasn't written (${FileViewError.toString(error)}),`
            + " so its look stands as it was");
        if (lookWriters[file] === writer && lookTexts[file] === writer.wrote && writer.was !== "")
            heardLook(file, writer.was);
        lookWritten(file, writer);
    }

    function lookWritten(file, writer) {
        if (lookWriters[file] === writer)
            delete lookWriters[file];
        writer.destroy();
    }

    // The session a terminal look is about, by the session or address it
    // names, or null when none runs.
    function lookSession(one) {
        return Sessions.running.find(session => ForYou.leftBy(one, session)) ?? null;
    }

    // Opening a look: a terminal's session gone to, anything else opened
    // by what opens its link or its file. Opened, it no longer needs Joel,
    // so a waiting one is written so and moves to History.
    function openLook(one) {
        if (one.kind === "terminal") {
            const session = lookSession(one);
            if (session !== null)
                Sessions.goTo(session);
        } else {
            Niri.spawn(["xdg-open", one.target]);
        }
        if (one.state === "waiting")
            settleLook(one, "opened", waitingSaid("openedHow", {}));
    }

    // A look as ForYou.js weighs it, blocking the session that left it
    // while that session waits on Joel (ForYou.js's lookThing).
    function lookThing(one) {
        return ForYou.lookThing(one, wantingHeard);
    }

    // A look's drawer: its quest, by its title in the index, and that it's
    // to look at, its title, what it weighs while it waits, since when,
    // what clears it, how it went once it has, its session, then who it is
    // and what it points at; and why, whole.
    function lookDrawer(one) {
        const now = Date.now();
        const say = forYouSaid(now);
        const waits = one.state === "waiting";
        const session = lookSession(one);
        const quest = one.quest ? WaitingOnYou.ownerOf(one.quest) : null;
        const from = quest?.title || one.quest || session?.title || one.session || "";
        const identified = (id, value) => Object.assign(fact(id, value), { identifies: true });
        const facts = [
            fact("weight", waits ? ForYou.weightSaid(lookThing(one), now, say) : ""),
            fact("since", one.since === null ? "" : say.clock(one.since)),
            fact("clears", !waits ? "" : one.kind === "pr" ? say.clearsPr : say.clearsLook),
            fact("resolved", waits ? "" : ForYou.historySide(one, say)),
            fact("session", ForYou.sessionSaid(session?.title ?? one.session ?? "",
                quest?.status)),
            identified("look", one.id),
            identified("target", ["pr", "page", "diff"].includes(one.kind) ? one.target : "")
        ].filter(each => each.value !== "");
        return {
            eyebrow: waitingSaid("from", { from: from, kind: say.look }),
            eyebrowNames: true,
            title: one.title,
            facts: facts,
            body: one.why
        };
    }

    // A look's row: its title, why in its note, its kind's mark, and at its
    // side the strongest reason it's there, or, in History, how it went and
    // when. Return opens it, or goes to a terminal's session; the right
    // arrow holds that, then Clear It while it waits, an ending act d does
    // and Return never does.
    function lookRow(one, now) {
        const say = forYouSaid(now);
        const waits = one.state === "waiting";
        const opens = one.kind === "terminal" ? "verb:on:waiting:go" : "verb:on:look:open";
        const inside = () => [thing({
            key: `verb:on:look:open:${one.id}`,
            verb: Words.verb(opens),
            name: Words.name(opens),
            glyph: Words.glyph(opens),
            hue: "action",
            later: true,
            run: () => provider.openLook(one)
        })].concat(waits ? [thing(Object.assign({
            key: `verb:on:look:clear:${one.id}`,
            verb: Words.verb("verb:on:look:clear"),
            name: Words.name("verb:on:look:clear"),
            glyph: Words.glyph("verb:on:look:clear"),
            hue: "action",
            stays: true,
            run: () => provider.settleLook(one, "cleared", waitingSaid("clearedHow", {}))
        }, Words.ending("verb:on:look:clear")))] : []);
        return member({
            thing: true,
            key: `on:look:${one.id}`,
            forYou: { kind: "look", source: "looks" },
            verb: Words.verb(opens),
            name: one.title,
            glyph: Theme.glyph[ForYou.glyphOf(one.kind)],
            hue: waits ? "strong" : "muted",
            side: lookSide(one, now, say),
            note: one.why,
            later: true,
            drawer: () => provider.lookDrawer(one),
            verbs: inside,
            targets: inside,
            run: () => provider.openLook(one)
        });
    }

    // What a look's row says at its side: the strongest reason it waits,
    // or, in History, how it went and when.
    function lookSide(one, now, say) {
        return one.state === "waiting" ? ForYou.reason(lookThing(one), now, say)
            : ForYou.historySide(one, say);
    }

    // A look's row, kept while its file reads the same and its side says
    // the same, which is all a row reads besides: For You is asked what
    // it holds as it's stepped into and again as anything in Now changes
    // while it's held, and making every look anew each time made 2,535
    // rows in one opening of Joel's (2026-11-09). Its side is what the
    // minute and who wants Joel change, and only for some looks, so the
    // rest are kept through both. Plain, so keeping notifies nothing.
    readonly property var looksKept: keeper()

    function keptLook(one, now) {
        const text = lookTexts[one.file];
        if (one.file === "" || typeof text !== "string")
            return lookRow(one, now);
        return keptAlike(looksKept, one.file, [text, lookSide(one, now, forYouSaid(now))],
            () => lookRow(one, now));
    }

    // The to-dos Joel said were done, while their quest's file
    // still tells of them (ForYou.js's settledItems).
    readonly property var doneItems: ForYou.settledItems(WaitingOnYou.asked,
        WaitingOnYou.settled, each => WaitingOnYou.identity(each))

    // A done item's row in History: its title, how it went and when at its
    // side, and its drawer as it had, with when it was resolved. Return
    // goes where it was done, while that's open.
    function doneRow(one) {
        const each = one.item;
        const say = forYouSaid(Date.now());
        const side = ForYou.historySide({ how: say.doneByYou, resolved: one.resolved,
            since: null }, say);
        return member({
            thing: true,
            key: `on:waiting:done:${WaitingOnYou.identity(each)}`,
            verb: Words.verb("verb:on:waiting:go"),
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: "muted",
            side: side,
            note: noteOf(each),
            later: true,
            drawer: () => {
                const drawn = provider.drawerOf(each);
                // What it weighed, when it was due and what would clear it
                // no longer matter once it's done, so how it went stands in
                // their place, as a look's does.
                const gone = ["weight", "due", "clears", "check"]
                    .map(id => Words.name(`on:waiting:${id}`));
                const facts = drawn.facts.filter(part => !gone.includes(part.label));
                const since = facts.findIndex(part => part.label
                    === Words.name("on:waiting:since"));
                facts.splice(since + 1, 0, fact("resolved", side));
                return Object.assign(drawn, { facts: facts });
            },
            run: () => WaitingOnYou.go(each)
        });
    }

    // Every look no longer waiting and every item done, the newest first,
    // one row at the very end of For You, so what's resolved can still be
    // found (Joel, 2026-11-08: "have a history so i can see anything
    // resolved in case i want to see it later").
    // Narrowed to a `source` of Lately's, only what came from it, as the
    // desktop's row ends on its own History (VKCF3Z, 2026-11-09).
    function historyRows(from) {
        const now = Date.now();
        const came = from === undefined ? cameDone
            : cameDone.filter(one => one.source === from);
        // Notices' rows of every notice done with, asked for at once, each
        // by its notice's id.
        const ids = came.map(one => one.of.id);
        const notices = {};
        for (const row of source("notices")?.rowsOf(ids, "history") ?? [])
            notices[row.notice] = row;
        const say = forYouSaid(now);
        const ours = from === undefined ? looks.history.map(one => ({
            at: one.resolved ?? one.since, row: () => provider.keptLook(one, now) }))
            .concat(doneItems.map(one => ({ at: one.resolved, row: () => provider.doneRow(one) })),
                given.done.map(one => ({ at: one.done.at,
                    row: () => provider.givenDone(one, say) })))
            : [];
        const rows = ours.concat(came.map(one => ({ at: one.at,
            row: () => provider.cameDoneRow(one, notices) })));
        return Order.sorted(rows, Order.stream(one => one.at)).map(one => one.row());
    }
    // What a source gave that's done, in History as its source made it,
    // saying at its side how it went and when, "read 14:02", as every row
    // there does.
    function givenDone(one, say) {
        return Object.assign({}, one.row, { side: ForYou.historySide({ how: one.done.how,
            resolved: one.done.at, since: null }, say), later: true });
    }
    readonly property int resolvedCount: looks.history.length + doneItems.length
        + cameDone.length + given.done.length
    readonly property var historyRow: whileWanted("history", () => resolvedCount === 0
        ? null : member({
        key: "on:waiting:history",
        verb: "more",
        stays: true,
        name: Words.name("on:waiting:history"),
        glyph: Words.glyph("on:waiting:history"),
        hue: "muted",
        side: provider.kept(resolvedCount),
        note: (Words.of("on:waiting:history").said ?? {}).note ?? "",
        empty: Words.of("on:waiting:history").empty,
        nested: true,
        verbs: () => provider.historyRows(),
        targets: () => provider.historyRows(),
        run: () => provider.launcher?.enter(provider.historyRow)
    }))

    // How many History keeps, said as what it counts, "1 kept", never a
    // bare number, and "none" while it keeps nothing, as every place says
    // (docs/design.md; VKCF3Z, 2026-11-09).
    function kept(count) {
        const said = Words.of("on:waiting:history").said ?? {};
        return count === 0 ? said.none ?? "" : ForYou.filled(said.count ?? "{count}",
            { count: count });
    }

    // History narrowed to what came from one source, the last row inside
    // its row, there with nothing in it too, so the way to what's read
    // never moves (VKCF3Z, 2026-11-09). It's a way elsewhere, so a place
    // holding only it says it holds nothing above it (Showing.js's
    // saysNothing).
    function historyOf(from) {
        const count = cameDone.filter(one => one.source === from).length;
        const row = member({
            key: `on:waiting:history:${from}`,
            verb: "more",
            stays: true,
            name: Words.name("on:waiting:history"),
            glyph: Words.glyph("on:waiting:history"),
            hue: "muted",
            side: provider.kept(count),
            empty: Words.of("on:waiting:history").empty,
            elsewhere: true,
            nested: true,
            verbs: () => provider.historyRows(from),
            targets: () => provider.historyRows(from),
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // Where a place named from outside leads, by the rows Joel would step
    // through from home: "notices", as Super+N with no cards and the
    // margin's Show Them ask, is For You's Desktop, one key to one place
    // whatever it holds (VKCF3Z, 2026-11-09), never the Notices place it
    // once was; the notices stay found by `/` (PastNotices.qml). "forYou",
    // as the margin's tray is pressed, is For You itself.
    function leadsTo(name) {
        if (name === "forYou")
            return [waiting];
        if (name !== "notices")
            return null;
        const { holding } = latelyBands(Date.now());
        return [waiting, keptNoticesRow("desktop",
            Lately.members(holding.desktop, one => one.who),
            { kind: "notice", source: "desktop" })];
    }

    // Everything that wants Joel, For You, one row however many: the pi
    // sessions that want him, each the row it is in Agents (Agents.qml's
    // made), whose Return goes to its window, what the agents have
    // waiting on him that he can do now, and what they left him to look
    // at, weighed by ForYou.js, the heaviest first, since what blocks a
    // session or is due bubbles up whatever kind it is (VKCF3Z,
    // 2026-11-08); the mark and the drawer's eyebrow say the kind. What
    // needs him now leads, one by one, and what's to look at or to try
    // and doesn't is each kind's one row after it (waitingRows). Then
    // what's for later and the history, each one row. It counts and
    // colours only what he can do now, its note names the first of
    // those, else the first for later. With
    // only later things, the row is muted and says how many. With nothing
    // waiting it's still there, muted, saying so, since Now's top never
    // changes shape (the launcher's shape, RSCH-20261028-2NK1E5, landing
    // 5). It's a place, so Return steps in, as the right arrow does, to the
    // most pressing first, or to a list saying nothing waits: the row acts
    // the same whatever waits, so Super+O, Return, Return reaches the
    // pressing thing and each press can be foreseen (VKCF3Z, 2026-10-29).
    // Not a session whose want is an action, though, which is one of its
    // own items (Holding.js's wanting). It's counted once, with the margin's
    // tray, in ForYouHolds.qml, so the two say one number (Y2YBTS,
    // 2026-11-09); here it's the same rule over the same sessions, as For
    // You hears them (wantingHeard).
    readonly property var sessions: Holding.wanting(wantingHeard)
    // The sessions wanting Joel, the same list while none of them moved
    // what For You reads (ForYou.js's toldOf), so a session working or
    // stepping, its want the same, weighs, bands and counts nothing again:
    // each move made For You's own row and every one of what's on's again,
    // and the node says one every few seconds (2026-11-10). What a row
    // does goes to the session as it is then (wantingNow).
    readonly property var wantingKept: ({ said: null, held: [] })
    readonly property var wantingHeard: {
        const wanting = Sessions.wanting;
        const said = wanting.map(session => ForYou.toldOf(session)).join("\n");
        if (said !== wantingKept.said) {
            wantingKept.said = said;
            wantingKept.held = wanting;
        }
        return wantingKept.held;
    }
    function wantingNow(session) {
        return Sessions.wanting.find(one => one.address === session.address) ?? session;
    }
    // What the launcher's other sources give For You, each its own row
    // made as its figures change (ForYou.js's given), as Passwords gives
    // its one to-do for what its last check found: read once as each
    // changes, those to do weighed beside the rest, those done in History.
    readonly property var given: ForYou.given(provider.launcher?.sources ?? [])
    // What the sources give, as Now reads them, counted with the rest where
    // the tray counts them too; and the looks, which the tray hears from the
    // to-do daemon instead wherever it runs (ForYouHolds.qml), both read
    // from the same folder.
    Binding {
        target: ForYouHolds
        property: "looks"
        value: provider.looks.waiting
    }
    Binding {
        target: ForYouHolds
        property: "given"
        value: provider.given.waiting
    }
    // What waits on Joel now, among it a critical notice unread, which
    // waits in For You's Desktop as the tray says (ForYouHolds.qml).
    readonly property int waitsNow: ForYouHolds.count
    readonly property var criticalNotices: ForYouHolds.holds.notices
    // The sessions there are to reopen, Agents' own row, drawn the same as
    // there, which For You lists as a thing to do while any are left (Joel
    // said yes to For You as the one place for what wants him, 2026-11-08),
    // and the tray counts as one (ForYouHolds.qml's reopen).
    readonly property var toReopen: source("agents")?.toReopen ?? null
    readonly property string first: toReopen !== null ? toReopen.name
        : sessions.length > 0
        ? `${sessions[0].title} ${Sessions.wanted(sessions[0].presence.wants)}`
        : WaitingOnYou.foremost ? WaitingOnYou.titled(WaitingOnYou.foremost)
        : looks.waiting.length > 0 ? looks.waiting[0].title
        : given.waiting.length > 0 ? given.waiting[0].row.name
        : criticalNotices.length > 0 ? String(criticalNotices[0].summary ?? "") : ""
    // What's inside, made and weighed as it's stepped into, so the order
    // is the one it had then; Steady.js keeps the rows still while it's
    // open. A session wanting him blocks one, itself.
    // A session that wants Joel, its row as Agents draws it, but Return steps
    // into what it asks rather than into the session, which may have moved
    // past it (Joel, 2026-11-08: "Selecting one brings me often into a pi
    // session that is so far past what i'm being asked that i don't know if
    // it is still relevant"). Inside, Deal With It brings its window forward
    // with the prompt filled in, naming what it asked, then Go To. One parked
    // is woken by either, its window showing it come back, as a key typed
    // there would; Deal With It wakes it once the filled prompt is in its
    // inbox, which it reads as it starts, as Talk does (Sessions.talk).
    // Each is kept while Agents' row for it, what it asked, its title and
    // whether it's parked are the same, as Agents keeps its own row while
    // its window and how it stands are (wantsKept).
    function wantRow(made, session) {
        // What it asked, in its own words, or the session by its name.
        const asked = String(session.presence?.wants?.said || session.title);
        const resting = Sessions.parkedAs(session.address);
        wantsNow[session.address] = session;
        return keptAlike(wantsKept, session.address, [made, asked, resting, session.title],
            () => madeWantRow(made, session, asked, resting));
    }
    readonly property var wantsKept: keeper()
    // Each session wanting Joel as For You last read it, which a kept
    // row's acts go to: Go To finds its window, its kitty and where it
    // runs from the session, which can move while its row reads the same
    // (Y2YBTS, 2026-11-10). Plain, so holding it notifies nothing.
    readonly property var wantsNow: ({})

    function madeWantRow(made, madeFrom, asked, resting) {
        const key = `on:waiting:want:${madeFrom.address}`;
        const address = madeFrom.address;
        const session = () => provider.wantsNow[address] ?? madeFrom;
        const named = id => ({
            verb: Words.verb(`verb:on:waiting:${id}`),
            name: Words.name(`verb:on:waiting:${id}`),
            glyph: Words.glyph(`verb:on:waiting:${id}`),
            hue: "action",
            side: madeFrom.title,
            later: true
        });
        const inside = () => [
            thing(Object.assign(named("deal"), {
                key: `verb:on:waiting:deal:${key}`,
                run: () => {
                    // The node wakes one parked once its words are in.
                    Sessions.steer(address, {
                        act: "fill",
                        text: readyToDeal(asked)
                    });
                    Sessions.goTo(session());
                }
            })),
            thing(Object.assign(named("go"), {
                key: `verb:on:waiting:go:${key}`,
                run: () => {
                    if (resting)
                        Sessions.wake(resting);
                    Sessions.goTo(session());
                }
            }))
        ];
        const row = member(Object.assign({}, made, {
            key: key,
            forYou: { kind: "do", source: "sessions" },
            // Its Return is stepping in, not Agents' own act on the session.
            returns: null,
            verb: "more",
            stays: true,
            verbs: inside,
            targets: inside,
            run: () => provider.launcher?.enter(row)
        }));
        return row;
    }

    // Everything For You weighs, each as ForYou.js bands it, { kind, thing,
    // make }, `make(now)` giving its row as it's stepped into: the sessions
    // to reopen, while any are; the sessions wanting Joel, each blocking
    // one, itself; what he can do now; what
    // he's to look at; and what the launcher's sources give, by the kind
    // its row says, a to-do where it says none. A session's row is Agents',
    // so without Agents there is none, and none is counted.
    readonly property var forYouItems: (toReopen === null ? [] : [{
            // Each session to reopen waits on him until he does, so the row
            // blocks as many as there are, and he can do it now: after a
            // boot it's the first thing to do.
            kind: "do",
            thing: { kind: "todo", ready: true, blocks: Sessions.toReopen.length },
            make: () => provider.toReopen
        }]).concat((provider.source("agents") ? sessions : [])
        .map(session => ({
            kind: "do",
            thing: { kind: "todo", blocks: 1, since: session.presence?.wants?.since ?? null },
            make: () => {
                const agents = provider.source("agents");
                const now = provider.wantingNow(session);
                const row = agents ? agents.made(now) : null;
                return row ? provider.wantRow(row, now) : null;
            }
        })).concat(WaitingOnYou.now.map(each => ({
            kind: "do",
            thing: provider.itemThing(each),
            make: now => provider.item(each, now)
        })), looks.waiting.map(one => ({
            kind: "look",
            thing: provider.lookThing(one),
            make: now => provider.keptLook(one, now)
        })), given.waiting.map(one => ({
            kind: one.row.forYou?.kind ?? "do",
            thing: one.thing,
            make: () => one.row
        }))))
    // How many each band holds, as they stand now (ForYou.js's banded):
    // what needs Joel now, lifted looks and things to try among them, then
    // what To Look At and To Try hold, which For You's row says as a run,
    // "1 to do \u00b7 44 to look at \u00b7 2 to try", and home's Now row says
    // the last two of while nothing needs Joel now (VKCF3Z, 3a92c450).
    // Being bound, they're banded at the moment the items last changed, so
    // For You's rows are banded again as it's stepped into a minute on,
    // since a deadline coming within the day lifts a row with no change.
    readonly property var forYouBands: bandsAt(Date.now())

    // For You's items banded, once for each change of them and each
    // minute: its count, its rows and its filter each banded them afresh,
    // three to five times for each act in it and each step into it, as
    // many milliseconds each as it holds items (2026-11-10). A deadline
    // passing is heard within the minute. The node's ready For You rows
    // (the agents daemon, Sat 14 Nov) take the banding over.
    function bandsAt(now) {
        const items = forYouItems;
        const minute = Math.floor(now / 60000);
        if (bandsKept.items !== items || bandsKept.minute !== minute) {
            bandsKept.items = items;
            bandsKept.minute = minute;
            bandsKept.bands = ForYou.banded(items, now);
            bandsKept.times += 1;
        }
        return bandsKept.bands;
    }
    readonly property var bandsKept: ({ items: null, minute: -1, bands: null, times: 0 })
    // A critical notice unread is among what needs him now, as the margin's
    // tray counts it (ForYouHolds.qml), though For You's Desktop holds it.
    readonly property int toDoCount: forYouBands.now.length + criticalNotices.length
    readonly property int toLookAtCount: forYouBands.look.length
    readonly property int toTryCount: forYouBands.try.length
    // For You's row's side, the bands as a run (ForYou.js's bandsRun).
    readonly property var bandsSaid: ForYou.bandsRun({ now: toDoCount, look: toLookAtCount,
        try: toTryCount }, Words.of("on:waiting").said ?? {})

    // The rows of a band, made now, as they're stepped into.
    function bandRows(band, now) {
        return band.map(one => one.make(now)).filter(row => row !== null);
    }

    // A kind's own row, To Look At or To Try, holding what of that kind
    // doesn't need Joel now (VKCF3Z, 2026-11-09; Joel: "there is no to look
    // at section"), none when it holds nothing: at its side how many, in
    // its note the heaviest's title, and inside each, heaviest first. It's
    // a place, so Return steps in, as the right arrow does, and its head
    // says "For You \u203a To Look At", as :filter look's does. It's a
    // place's row, so nothing stands beside its name and its side is a bare
    // count; its drawer names the heaviest. Its mark is muted, as any row
    // that holds more draws one, its name in text, so what needs him now
    // leads by its order (VKCF3Z, 2026-11-09).
    // Kept while it holds the same rows, each kept as it is (kindsKept).
    function kindRow(kind, rows) {
        if (rows.length === 0)
            return [];
        return [keptAlike(kindsKept, kind, rows, () => madeKindRow(kind, rows))];
    }
    readonly property var kindsKept: keeper()

    function madeKindRow(kind, rows) {
        const id = `for-you:kind:${kind}`;
        const row = member({
            key: `on:waitingOnYou:kind:${kind}`,
            verb: "more",
            stays: true,
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "muted",
            side: String(rows.length),
            nested: true,
            thing: true,
            drawer: () => ({
                title: Words.name(id),
                facts: [provider.fact("heaviest", rows[0].name)]
            }),
            verbs: () => rows,
            targets: () => rows,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // For You's own rows, each as it's for (ForYou.js's bands), the
    // heaviest first in each: what needs Joel now, one by one, then To Look
    // At and To Try, then Lately, Later and History (VKCF3Z, 2026-11-09).
    // Each asking is a pass of the to-dos', sessions' and kinds' keeping,
    // so what's no longer there is let go; a look is let go once its file
    // is, since History's looks are kept beside them.
    function waitingRows() {
        const now = Date.now();
        const bands = bandsAt(now);
        const keeps = [itemsKept, wantsKept, kindsKept];
        keeps.forEach(keeping => keepingBegun(keeping));
        for (const identity of Object.keys(itemsNow))
            delete itemsNow[identity];
        for (const address of Object.keys(wantsNow))
            delete wantsNow[address];
        const rows = bandRows(bands.now, now).concat(kindRow("look", bandRows(bands.look, now)),
            kindRow("try", bandRows(bands.try, now)), latelyRows(now),
            laterRow === null ? [] : [laterRow], historyRow === null ? [] : [historyRow]);
        keeps.forEach(keeping => keptOnly(keeping));
        for (const file of Object.keys(looksKept.rows)) {
            if (!(file in lookTexts))
                delete looksKept.rows[file];
        }
        return rows;
    }

    // Every row For You's :filter narrows, each one by one, as its bands
    // order them: a filter steps past the rows that hold a kind, so
    // :filter look holds what To Look At does, and any lifted (VKCF3Z,
    // 2026-11-09).
    function filterable() {
        const now = Date.now();
        const bands = bandsAt(now);
        return bandRows(bands.now, now).concat(bandRows(bands.look, now),
            bandRows(bands.try, now), latelyRows(now));
    }

    // What came for Joel, For You's Lately band, after what he can do and
    // before what's for later (Y2YBTS, 2026-11-08; Joel: "a notice stays
    // until read or forgotten"): a row for each source holding something
    // unread. Mail and Slack are their pieces' rows (NewMail.qml,
    // NewSlack.qml), each piece saying what it holds (Piece.qml's lately
    // and came), and Desktop is the notices unread, the shell's own for
    // what For You lists left out. The rows stand by the heaviest each
    // holds, a thing to look at weighed by how long it's waited, as For
    // You weighs one, and each note names who it's from, the newest first
    // (VKCF3Z, 2026-11-08). Each leaves once its source says all it held
    // is done, and what's done goes to History.
    readonly property var latelyPieces: Facts.all.filter(fact => fact.piece
        && Lately.SOURCES.includes(fact.piece.lately) && fact.piece.present
        && fact.piece.active)
    // The notices as For You holds them: one read since the launcher
    // opened is still unread here, so Desktop holds still while Joel looks
    // at it (VKCF3Z, 2026-11-08) and u brings back what d did there; it
    // goes to History once the launcher closes. One withdrawn goes at once.
    readonly property var latelyNotices: Lately.holdingStill(Notices.history, openedAt)
    // The sources whose own piece says what's done with by itself, Slack
    // while its daemon answers whether or not anything's unread, so their
    // notices stand for nothing (Piece.qml's answers).
    readonly property var answering: Facts.all.filter(fact => fact.piece?.present
        && fact.piece.answers).map(fact => fact.piece.lately)
    // The unread notices of each source (Lately.js's held): the desktop's,
    // and Slack's, which stand for Slack's row only while its daemon can't.
    readonly property var heldNotices: Lately.held(latelyNotices, answering)
    readonly property var unreadNotices: heldNotices.desktop
    readonly property var noticesOf: heldNotices

    // Desktop and History are For You's, which is Now's, so a notice
    // forgotten, brought back or read is said again in them: the launcher
    // makes a place again only when what it was entered from changes.
    Connections {
        target: Notices

        function onHistoryChanged() {
            provider.changed();
        }
    }

    // What each source holds, { holding, bands }: who each thing it holds
    // is from and when it came, newest first, and Lately's rows, heaviest
    // first (Lately.js's bands).
    function latelyBands(now) {
        const holding = {};
        for (const source of Lately.SOURCES)
            holding[source] = (noticesOf[source] ?? []).map(notice => ({ who: notice.app,
                at: notice.at }));
        for (const fact of latelyPieces)
            holding[fact.piece.lately] = holding[fact.piece.lately].concat(fact.piece.came);
        for (const source of Lately.SOURCES)
            holding[source] = Order.sorted(holding[source].map(one => ({ who: one.who,
                at: Lately.when(one.at) })), Order.stream(one => one.at));
        const weigh = one => ForYou.weigh({ kind: "look", ready: true, since: one.at }, now)
            .weight;
        return { holding, bands: Lately.bands(holding, weigh) };
    }

    // What For You's drawer names as its heaviest (ForYou.js's heaviest).
    function heaviestSaid() {
        return ForYou.heaviest(toDoCount > 0 ? homeItems() : [],
            forYouBands.look[0]?.make(Date.now())?.name);
    }

    // Each is a place's row, stepped into for rows of its own (VKCF3Z,
    // 2026-11-09, fef0c4d5): nothing after its name, as no place says
    // there since #2600, its side how many to read, and its drawer a
    // place's, naming who it holds is from, the newest first, as a run that
    // wraps between names (PlaceRow.js's drawer). It names them as
    // `heldNames`, as a place inside another does, which is what gives a
    // row that's no thing its drawer (Launcher.qml's drawable and
    // heldDrawer); a drawer of its own was held for one never shown.
    // Each is kept while what it says reads the same, a piece's by what
    // its piece shows (pieceSays) and Desktop's by the notices it holds,
    // with who they're from: each asking of For You made every one anew,
    // a piece's acts too, 7 rows an asking with nothing changed
    // (2026-11-10). A kept piece's acts read the piece as it is now
    // (latelyNow), as what's on reads piecesNow.
    function latelyRows(now) {
        const { holding, bands } = latelyBands(now);
        keepingBegun(latelyKept);
        for (const mark of Object.keys(latelyNow))
            delete latelyNow[mark];
        const rows = bands.map(band => {
            const senders = Lately.members(holding[band.source], one => one.who);
            const forYou = { kind: "notice", source: band.source };
            const fact = latelyPieces.find(one => one.piece.lately === band.source);
            if (fact === undefined)
                return provider.keptNoticesRow(band.source, senders, forYou);
            latelyNow[fact.entry.mark] = fact.piece;
            return keptAlike(latelyKept, `piece:${band.source}`, [pieceSays(fact),
                senders.join("\u241f")], () => made(fact, { forYou: forYou, count: "", note: "",
                heldNames: senders }));
        });
        keptOnly(latelyKept);
        return rows;
    }
    readonly property var latelyKept: keeper()
    readonly property var latelyNow: ({})

    // A source's unread notices' row (noticesRow), kept while it holds the
    // same notices from the same senders and its History as many, which is
    // all it and what it holds of its own say.
    function keptNoticesRow(from, senders, forYou) {
        const ids = (noticesOf[from] ?? []).map(notice => notice.id).join(" ");
        const done = cameDone.filter(one => one.source === from).length;
        return keptAlike(latelyKept, `notices:${from}`, [ids, senders.join("\u241f"), done],
            () => noticesRow(from, senders, forYou));
    }

    // A Lately source's drawer, a place's: its eyebrow saying so, as every
    // place's does, and who it holds is from.
    function latelyDrawer(name, senders) {
        return PlaceRow.drawer(name, senders, (Words.field.drawer ?? {}).place ?? "");
    }

    // A source's unread notices' row, the desktop's, or Slack's while its
    // piece can't speak: how many to read, and its drawer from whom.
    // Return steps in to each, the row it is in
    // Notices, then Mark Them Read, and last its History, what's read.
    // With none unread it says so where an empty place does, over its
    // History alone, the cursor on it: a row Return does nothing on would
    // be a fake one (VKCF3Z, 2026-11-09). Stepping in
    // counts them seen, so the margin no longer says any were missed; `:`
    // listing its acts doesn't. It's after the step, so the history isn't
    // changed while it's read.
    function noticesRow(from, senders, forYou) {
        const words = Words.of(`for-you:source:${from}`);
        const said = Words.of("for-you:source:desktop").said ?? {};
        const notices = noticesOf[from] ?? [];
        const ids = notices.map(notice => notice.id);
        const act = (key, words, more) => thing(Object.assign({
            key: key,
            verb: Words.verb(words),
            name: Words.name(words),
            glyph: Words.glyph(words),
            hue: "action"
        }, more));
        const none = words.empty ?? "";
        // Its own act and its History made once, as it's first stepped
        // into, since the row is made again whenever the notices it holds
        // change.
        let own = null;
        const owned = () => own ?? (own = (ids.length === 0 ? [] : [
            act(`verb:on:waiting:${from}:read`, "verb:on:waiting:desktop:read", {
                stays: true,
                run: () => Notices.markRead(ids)
            })]).concat([provider.historyOf(from)]));
        const listed = () => (ids.length > 0 ? (source("notices")?.rowsOf(ids, "desktop") ?? [])
            : []).concat(owned());
        const row = member({
            key: `on:waiting:${from}`,
            forYou: forYou,
            verb: "more",
            stays: true,
            name: Words.name(`for-you:source:${from}`),
            glyph: Words.glyph(`for-you:source:${from}`),
            hue: "muted",
            side: ForYou.filled(said.count ?? "{count}", { count: ids.length }),
            empty: none,
            nested: true,
            thing: true,
            drawer: () => latelyDrawer(Words.name(`for-you:source:${from}`), senders),
            verbs: listed,
            targets: listed,
            stepped: () => Qt.callLater(() => Notices.seen()),
            // Held, the desktop's lists the notices the screen shows, so the
            // cards step aside while it's open (PastNotices.qml), each in one
            // place; Slack's, standing in while its daemon doesn't answer,
            // lists Slack's alone, so the rest stay on screen.
            gathers: from === "desktop",
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // What came and is done with, each { title, how, at, source, of }, for
    // History (Lately.js's done): a notice read here, or withdrawn by its
    // sender, as "read here 14:02" (VKCF3Z, 2026-11-08), mail read or
    // archived in Gmail or here, as Google's daemon says, and a Slack
    // message read in Slack, as its daemon says, "read in Slack 14:02",
    // while the daemon holds every one (Piece.qml's finished), each how
    // worded from its source's words. A piece holding nothing more to read
    // is asked too, since what it finished outlasts its row.
    function finishedOf(source) {
        return Facts.all.find(fact => fact.piece?.lately === source && fact.piece.present)
            ?.piece.finished ?? null;
    }
    readonly property var cameDone: Lately.done({ notices: latelyNotices,
        messages: finishedOf("slack"), finished: finishedOf("mail") ?? [] },
        { mail: Words.of("mail:message").said ?? {},
            slack: Words.of("slack:message").said ?? {},
            notice: Words.of("for-you:source:desktop").said ?? {} })

    // A done one's row in History: the row its piece gives it, as a piece
    // of mail's or a Slack message's, or a notice's row in Notices, with
    // how and when it was done with at its side. `rows` are Notices' rows
    // of every notice done with, by its id, asked for at once (historyRows).
    function cameDoneRow(one, rows) {
        const side = ForYou.historySide({ how: one.how, resolved: one.at, since: null },
            forYouSaid(Date.now()));
        const given = typeof one.of.row === "function" ? one.of.row(thing) : null;
        const row = given ?? (one.source === "mail" ? null : rows[one.of.id]) ?? member({
            key: `on:waiting:came:${one.source}:${one.at}`,
            name: one.title,
            glyph: Words.glyph("for-you:source:desktop"),
            plain: true
        });
        // Its side is how it went and when, never the notice's own items.
        return Object.assign({}, row, { side: side, sideItems: [], hue: "muted",
            forYou: { kind: "notice", source: one.source } });
    }
    readonly property var waiting: whileWanted("waiting", () => {
        const glyph = Theme.glyph[ForYou.markOf(provider.waitsNow, WaitingOnYou.later.length)];
        const hue = provider.waitsNow > 0 ? "strong" : "muted";
        const bands = provider.bandsSaid;
        const side = bands.length > 0 ? bands.map(item => item.said).join(" \u00b7 ")
            : WaitingOnYou.later.length > 0
                ? waitingSaid("later", { count: WaitingOnYou.later.length })
                : waitingSaid("none", {});
        return keptAlike(ownKept, "waiting", [glyph, hue, side, JSON.stringify(bands)],
            () => madeWaiting(glyph, hue, bands, side));
    })

    function madeWaiting(glyph, hue, bands, side) {
        return member({
            // Not `on:waiting`, which its rows within are keyed under
            // (Marks.js's ROW_MARKS, which point both at For You's tray).
            key: "on:waitingOnYou",
            verb: "more",
            stays: true,
            // What its list says when it holds nothing (Launcher.qml).
            empty: Words.of("on:waiting").empty,
            name: Words.name("on:waiting"),
            glyph: glyph,
            hue: hue,
            // How many each band holds, "1 to do \u00b7 44 to look at \u00b7 2
            // to try", or with none, whether anything comes later (VKCF3Z,
            // 3a92c450).
            sideItems: bands,
            side: side,
            // A place's row, so nothing stands after its name; its drawer
            // names the heaviest it holds, as To Look At's does, and the
            // bands keep the row's room (VKCF3Z, 16efb1dc). Holding only
            // things to try, it names none, so it has no drawer.
            nested: true,
            thing: true,
            drawer: () => {
                const heaviest = heaviestSaid();
                return heaviest === "" ? null : {
                    title: Words.name("on:waiting"),
                    facts: [provider.fact("heaviest", heaviest)]
                };
            },
            verbs: () => provider.waitingRows(),
            targets: () => provider.waitingRows(),
            commands: () => provider.filterCommands(null),
            run: () => provider.launcher?.enter(provider.waiting)
        });
    }

    // What For You lists from, so a change in any while For You or a step
    // in it is held asks whether what's held reads otherwise (changedTo),
    // since its own row, kept while its count reads the same, no longer
    // moves all of Now for it. Asked after the change, never inside it.
    readonly property var forYouHeard: [forYouItems, WaitingOnYou.later, given, latelyPieces,
        Sessions.wanting]
    onForYouHeardChanged: Qt.callLater(Hearing.putOff(forYouTold))
    // Timed from the earliest line whose change it tells (Hearing.js).
    readonly property var forYouTold: Hearing.carried(() => provider.forYouMoved())

    function forYouMoved() {
        if (heldKeys().length > 0)
            changedTo(all);
    }

    // The words a filter's kinds and sources are said in, for its head.
    readonly property var filterSaid: Filter.saidFrom(Object.keys(Words.things),
        id => Words.name(id))

    // For You's :filter, from For You or from a filter inside it, `from`:
    // a step into the rows of a kind, a source or both (ForYouFilter.js),
    // as VKCF3Z ruled it (2026-11-08). From inside a filter it narrows that
    // one and takes its place, so `-` is always one step back out to
    // everything. Tab offers the kinds and sources there, with how many.
    // The rows are read once, as the line opens, so a key typed after it
    // weighs none of them again, as :set's options are (Launcher.qml's
    // standingOptions).
    function filterCommands(from) {
        const rows = filterable();
        return [{
                command: "filter",
                short: "filt",
                title: (Words.field.command ?? {}).filter ?? "",
                bang: false,
                args: arg => Filter.offered(rows, arg, filterSaid, from)
                    .map(one => Object.assign({ glyph: filterGlyph(one) }, one)),
                run: call => {
                    const filter = Filter.asked(call.arg, rows, filterSaid, from);
                    if (filter.error !== undefined) {
                        provider.launcher?.commandSay(filter.error, true);
                        return;
                    }
                    if (from !== null)
                        provider.launcher?.back();
                    provider.launcher?.enter(filterRow(filter));
                }
            }];
    }

    // The mark a kind or source wears in :filter's menu (ForYouFilter.js).
    function filterGlyph(one) {
        const mark = Filter.markOf(one);
        return mark.program !== undefined ? provider.programGlyph(mark.program)
            : Theme.glyph[mark.glyph] ?? "";
    }

    // A filter's place, named for what it keeps, which the head shows after
    // For You. What it holds is For You's rows as they stand when it's
    // stepped into, in For You's order.
    function filterRow(filter) {
        const rows = () => Filter.filtered(filterable(), filter);
        const row = member({
            key: `on:waitingOnYou:filter:${Filter.keyOf(filter)}`,
            verb: "more",
            stays: true,
            empty: Words.of("on:waiting").empty,
            name: Filter.heading(filter, filterSaid),
            // For You's own tray, as its head is For You's with a filter.
            glyph: Theme.glyph[ForYou.markOf(provider.waitsNow, WaitingOnYou.later.length)],
            hue: "strong",
            nested: true,
            verbs: rows,
            targets: rows,
            commands: () => filterCommands(filter),
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }
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

    // The drawer of a place's row a piece names as its own (Piece.qml's
    // drawnBy), as that place draws it, or none while it doesn't list it:
    // a place keeps its rows in `all` where it works them out, as Sound
    // does, else in a provider's `things`, as Cameras does.
    // The drawer of the row a piece stands for, without what the piece's
    // own row says (Showing.besides), as `by` gives it.
    // A place whose rows aren't wanted, as Sound's aren't while Now is
    // held, makes the one row as it reads now (Provider.qml's rowNow).
    function borrowed(drawnBy, by) {
        const place = source(drawnBy.place);
        const now = typeof place?.rowNow === "function" ? place.rowNow(drawnBy.key) : undefined;
        const row = now !== undefined ? now
            : (place?.all ?? place?.things ?? []).find(each => each.key === drawnBy.key);
        // A drawer made as it's drawn, or one a row holds made, as
        // Power's parts do (Launcher.qml's drawerOf takes both).
        const drawer = typeof row?.drawer === "function" ? row.drawer() : row?.drawer ?? null;
        return Showing.besides(drawer, by ?? {});
    }

    // `over` holds what a row made elsewhere says in place of the piece's
    // own, as For You's Lately rows do.
    function made(fact, over) {
        const piece = fact.piece;
        const now = () => piecesNow[fact.entry.mark] ?? latelyNow[fact.entry.mark] ?? piece;
        // Every act a piece has, where it keeps more than its notice can
        // offer (pieces/Pi.qml's every).
        const actions = (piece.every ?? piece.actions).map((action, at) => thing({
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
            // The piece's act as it is when run, since its row may be kept
            // while the act was made again saying the same.
            run: () => ((now().every ?? now().actions)[at] ?? action).run(provider.shell)
        }));
        // A piece standing for things of the launcher's own, such as the
        // mail that wants Joel, lists them on Return, each the row it is
        // everywhere else, and its own acts after them (Piece.qml's rows).
        // Its rows say what they hold after their names as its own row
        // does, giving way by whole items (listedAfter).
        const holding = piece.rows ? () => now().rows(thing, listedAfter).concat(actions) : null;
        // Everything its source says, under the row, where the piece gives
        // it (Piece.qml's drawer), as the network allows, read again as the
        // drawn clock turns. Its note is said there, once, while the drawer
        // is out, so its side reads on the row as its neighbours' do
        // (Showing.noted; VKCF3Z, 2026-11-09).
        const drawn = typeof piece.drawer === "function" ? {
            thing: true,
            noteInDrawer: true,
            drawer: () => {
                void drawnClock.date;
                return Showing.noted(now().drawer({ lookups: settings.lookups !== false }),
                    now().context);
            }
        } : piece.drawnBy ? {
            thing: true,
            noteInDrawer: true,
            // Which row it draws, so a place reading more of a thing
            // while it's selected, as Wi-Fi does, reads it here too.
            borrows: piece.drawnBy,
            drawer: () => Showing.noted(provider.borrowed(now().drawnBy, {
                side: now().value,
                note: now().context
            }), now().context)
        } : {};
        return member(Object.assign({
            key: `on:${fact.entry.mark}`,
            is: provider.same[fact.entry.mark] ?? `on:${fact.entry.mark}`,
            // What Return does, said in the foot: the first action's own
            // verb where it gives one, else its name ("let it sleep").
            verb: piece.actions.length === 0 ? "close"
                : piece.actions[0].verb ?? piece.actions[0].name.toLowerCase(),
            name: piece.title,
            side: piece.value,
            sideItems: piece.valueItems ?? [],
            counts: piece.counts ?? undefined,
            // A thing's detail after its name, as a call names who's in
            // it; a piece that's a place says nothing there, naming what
            // it holds in its drawer instead (VKCF3Z, 5fa2cbda).
            count: parted(piece),
            heldNames: piece.heldNames ?? undefined,
            sideWidest: piece.widest,
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
        }, drawn, over ?? {}));
    }

    // A piece's parts as its row says them after its name: as many whole
    // names as the row leaves room for beside its name and its side, cut
    // from the end as a place's are (Launcher.qml's listed), or all of them
    // where no launcher says the room.
    function parted(piece) {
        return listedAfter(piece.parts ?? [], piece.title, piece.value);
    }

    // Parts said after a row's name, in the room its name and side leave,
    // giving way by whole parts.
    function listedAfter(parts, name, side) {
        const launcher = provider.launcher;
        if (parts.length === 0 || !launcher)
            return parts.join(", ");
        const beside = name.length + side.length + "  ".length;
        return launcher.listed(parts, launcher.rowRoom() - beside);
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

    // Now's row says at its right how Now stands, what waits on Joel first:
    // the thing For You's own row leads with and its minutes, then how many
    // more need him now, "Answer the ask \u00b7 2 min \u00b7 2 more"; with
    // nothing needing him now, what To Look At and To Try hold, "44 to look
    // at \u00b7 2 to try"; and with none of those, nothing, since a calm home
    // says nothing (VKCF3Z, bd5cb965, a31a0e1b, 3a92c450). For You is his
    // one place for attention, so home leads with it.
    function homeSays() {
        return homeItems().map(item => item.said).join(" \u00b7 ");
    }

    // Never how many rows it holds, which tells Joel nothing: calm, its
    // right is empty (VKCF3Z, bd5cb965).
    function counted() {
        return "";
    }

    // The same, as items: short of room its title gives way from its end
    // by whole words, behind an ellipsis, and its minutes and how many more
    // never give way (Reading.js's keptItems).
    // It leads with what needs him now, never a later item as `first`
    // falls back to, so "N more" counts the rest of what needs him now,
    // never what To Look At and To Try hold.
    function homeItems() {
        if (toDoCount === 0) {
            return ForYou.bandsRun({ look: toLookAtCount, try: toTryCount },
                Words.of("on:waiting").said ?? {});
        }
        const lead = sessions.length === 0 ? WaitingOnYou.now[0] ?? null : null;
        // A look or a landing lifted into what needs him now, where no
        // session or to-do leads, says its own row's name.
        const lifted = sessions.length > 0 || lead ? null : forYouBands.now[0] ?? null;
        const title = sessions.length > 0 ? first : lead ? lead.title
            : lifted?.make(Date.now())?.name
            ?? (criticalNotices.length > 0 ? String(criticalNotices[0].summary ?? "") : "");
        if (title === "") {
            return [];
        }
        const minutes = typeof lead?.minutes === "number"
            ? waitingSaid("minutes", { minutes: lead.minutes }) : "";
        const more = toDoCount - 1;
        return [{ said: title, keep: 1, words: true, stays: true },
            { said: minutes, keep: 2, stays: true },
            { said: more > 0 ? waitingSaid("more", { count: more }) : "", keep: 3, stays: true }]
            .filter(item => item.said !== "");
    }

    // What its drawer names, the rest of what's on, never For You, whose
    // first thing it already says at its right.
    function memberNames(held = all) {
        return held.filter(one => one.key !== waiting.key && one.folded !== true)
            .map(one => one.name);
    }

    // The key of the row above, Now's top, that already stands for what's
    // on, or "": never a second row beside it (Joel, 2026-10-28: "seeing
    // multiples of the same thing gives me executive dysfunction"). The
    // sessions wanting Joel are in Waiting on You. The meeting coming
    // (Starting.qml) is the calendar's next one but for the few minutes one
    // runs on past its start, and is then the moment's meeting row.
    function above(fact) {
        // What came for Joel is For You's Lately, and the notices missed
        // are among its Desktop row's (latelyRows). The margin's tray is
        // For You's own.
        if (fact.entry.mark === "forYou" || Lately.SOURCES.includes(fact.piece.lately))
            return waiting.key;
        if (fact.entry.mark === "meeting" && meeting !== null
            && meeting.key === `calendar:meeting:${fact.piece.meeting?.id ?? ""}`)
            return meeting.key;
        return "";
    }
}
