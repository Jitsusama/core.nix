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
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../Hearing.js" as Hearing
import "../Marks.js" as Marks
import "../When.js" as When
import "Ago.js" as Ago
import "ForYou.js" as ForYou
import "ForYouCells.js" as Cells
import "ForYouFilter.js" as Filter
import "Lately.js" as Lately
import "Looks.js" as Looks
import "NowGroups.js" as Groups
import "Showing.js" as Showing
import "Order.js" as Order
import "Moment.js" as Moment
import "NeedsYou.js" as NeedsYou
import "Here.js" as Here
import "PlaceRow.js" as PlaceRow
import "SessionRow.js" as SessionRow
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
        // there, so the top never moves; then what came for him that asks
        // nothing of him (latelyInForYou); then the rest by group, what wants
        // Joel first, and by name within each, so a row keeps its place
        // while its group stands (NowGroups.js).
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
        // 2026-11-10). For You's row and the location are whileWanted's
        // own, so each is read through `standing`, which makes it as it
        // stands when the launcher lists Now shut (Provider's asItStands).
        return [standing("waiting", waiting)].concat(momentAt(standing("location", location)),
            latelyRows(Date.now()).filter(row => !latelyInForYou(row)),
            Groups.ordered(rows).map(each => each.row));
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

    // An item with neither a window nor a session here is said in the foot where Go To or
    // Deal With It was pressed, never a press that does nothing (Joel,
    // 2026-11-17 00:12: "when i hit deal with it when a window is not
    // loaded from the launcher, nothing happens"). What goTo itself can't
    // open, Agents says (Sessions' unjoined, unhosted and unopened).
    Connections {
        target: WaitingOnYou

        function onUnfound(item) {
            provider.launcher?.tellOnOpen([[Words.name("on:waiting"),
                waitingSaid("unfound", {}), Theme.alert]]);
        }
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                // Shut before Now woke, when it opened never left infinity,
                // so nothing else puts Desktop back where a shut launcher
                // keeps it (replaceLately), and the next opening would
                // start from where this one left it.
                const neverOpened = provider.openedAt === Number.POSITIVE_INFINITY;
                provider.openingAt = -1;
                provider.openedAt = Number.POSITIVE_INFINITY;
                if (neverOpened)
                    provider.replaceLately();
                return;
            }
            provider.openingAt = Date.now();
            if (!provider.asleep)
                provider.feltOpening(() => provider.opens());
        }
    }

    // When the launcher opened while Now slept, kept till Now wakes in its
    // turn (Launcher.qml's wakeNext), so its opening work runs in its own
    // slice, not in the turn the launcher opened in: the bindings on when
    // it opened, the notices over every one held, ran there inside the
    // launcher's unnamed "opening", about 12 of its 100 ms over twelve of
    // the keys lab's openings (its profile, 2026-11-20). -1 when shut, or
    // once its opening work has run.
    property real openingAt: -1
    onAsleepChanged: {
        if (!asleep)
            opensIfWaiting();
    }
    // Its opening work run now, while it waits for Now to wake in an open
    // launcher, as the notices' history is given again, which every change
    // the daemon sends does. A notice come or read before then would be
    // held still from no opening (latelyNotices), so a row read under the
    // cursor would drop and come back as Now woke.
    function opensIfWaiting() {
        if (openingAt >= 0 && provider.launcher?.open)
            provider.feltOpening(() => provider.opens());
    }
    Connections {
        target: Notices

        function onHistoryChanged() {
            provider.opensIfWaiting();
        }
    }

    // Its work as the launcher opens, felt whole: the bindings on when it
    // opened and the hour its looks are split by run as they're set. When
    // it opened is when the launcher did, however late Now woke, so a
    // notice read since still holds still.
    function opens() {
        // Taken before it's set, so what setting it runs finds none waiting.
        const kept = provider.openingAt;
        provider.openingAt = -1;
        provider.openedAt = kept >= 0 ? kept : Date.now();
        provider.looksHour = Math.floor(Date.now() / 3600000);
        provider.reweather();
        // Placing the time and the date makes Time's and Today's rows, so
        // an opening into a place that shows none of Now's leaves them
        // stale until Now is wanted (H6EDLC, 2026-11-11), and one onto
        // Now's rows places them only once a minute has turned since they
        // were placed: each opening into For You made all 17 again with
        // nothing changed (2026-11-11). An opening elsewhere leaves them
        // for when Now's rows are wanted (onWantedChanged), which asks the
        // same: marking them stale as it opened made all 17 again within
        // the minute, whenever it came before Now fell asleep, as a loaded
        // machine's openings into For You did (2026-11-12).
        if (provider.wanted)
            provider.retimeIfStale();
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

    // Whether the clock said a minute turned while Now's rows weren't
    // wanted, so it's read as they are again. Placing the time and
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

        // Said for the opening test, which reopens For You only within a
        // minute the shell heard no turn of, since a turn rightly makes
        // Time's and Today's rows again however late it's heard.
        function onDateChanged() {
            provider.minuteTurned();
        }
    }

    // A turn heard after the time was placed in the minute it turned to
    // leaves nothing stale: a loaded machine can hear the clock's turn
    // late, after an opening placed it, and marking it stale then made
    // Time's and Today's rows again within the minute as Now's rows were
    // come to (the-launcher-places-the-time-only-when-now-is-wanted,
    // "said 1, meant 0", 2026-11-12).
    function minuteTurned() {
        Report.say("the clock turned a minute");
        if (timedMinute === minuteNow())
            return;
        if (wanted)
            Qt.callLater(retime);
        else
            timeStale = true;
    }

    // The looks the stream already holds, as a launcher made after the
    // shell's first answer hears no change until the next.
    Component.onCompleted: {
        Qt.callLater(retime);
        Qt.callLater(looksHeard);
        Qt.callLater(armLift);
        replaceLately();
    }
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

    // This moment's rows, with `here` as the location: `all`, which reads
    // them, is made again as the clock turns a minute, so the time says it.
    function momentAt(here) {
        const rows = [timeRow, dateRow, here];
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
        // Chrony is asked only while home shows, so until the reader
        // answers it's left out; that nothing keeps the time is said only
        // once the reader refused, as on a machine without chrony.
        const time = source("time");
        const kept = time?.kept ?? null;
        const offset = kept?.exchange ? time.millis(kept.exchange.offset, true) : "";
        const step = kept ? said(kept.synchronised ? "synchronised" : "unsynchronised")
            : Timekeeping.refused ? said("unkept") : "";
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
            identifier("address", owner.address, false),
            identifier("check", owner.check, true),
            Object.assign(fact("item", WaitingOnYou.identity(each)), { identifies: true })
        ].filter(one => one.value !== "");
        return {
            // Its quest by its title (VKCF3Z, 2026-11-08), and how it
            // stands where it isn't active, in place of a Session fact
            // saying it again (Y2YBTS, 2026-11-12); its owner's address is
            // a fact of its own.
            eyebrow: waitingSaid("from", { from: ForYou.sessionSaid(quest || session?.title || "",
                WaitingOnYou.owners[each.quest]?.status), kind: say.todo }),
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
    // valid"). The harness fills a prompt only while it's empty. Going
    // comes first, so a parked session's wake reaches its inbox before
    // the fill it then reads.
    function dealWith(each) {
        const session = WaitingOnYou.session(each);
        WaitingOnYou.go(each);
        if (session)
            Sessions.steer(session.address, {
                act: "fill",
                text: readyToDeal(each.title)
            });
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
    // window's own title, else the session it's from in no window, which
    // Go To opens one on (WaitingOnYou.go); "" when it has neither.
    function whereOf(each) {
        const window = WaitingOnYou.windowOf(each);
        const own = WaitingOnYou.session(each);
        if (window === null)
            return own?.title ?? "";
        if (own !== null && own.window?.id === window.id)
            return own.title;
        // A session in no window, as one held or elsewhere is, isn't it.
        const session = Sessions.running.find(one => one.window?.id === window.id);
        return session ? session.title : window.title ?? "";
    }

    // Talk to a session, Agents' own act, as a look's row offers it
    // (cellActs), so the session working on what Joel's asked is reached
    // from the ask itself (Joel, 2026-11-16: "i have no way to go from the
    // thing it shows to engaging with the thing working on it"); null with
    // no session, or with Agents not there to talk through.
    // A session asking a question or a yes has no Talk from any row in What
    // Needs Me (Agents' own row is its own to say): its ask tool runs until
    // he answers, and words said to it wait for its run to end (the
    // harness's lib/mesh/steer.ts), so they'd come after the answer they
    // were meant to shape; its answer is what reaches it, In Your Own Words
    // where its question takes words. Parked, it runs no ask, and woken it
    // reads its inbox as it starts, so Talk stays.
    function talkTo(session) {
        const agents = provider.source("agents");
        return !session || !agents || asking(session) ? null
            : agents.sessionAct(session, "talk");
    }

    // Whether a session wanting Joel asks him something still open, a
    // question or a yes (the harness's WantKind), rather than having
    // ended blocked: the kinds the agents daemon rings for (wants.rs's
    // rings), which keep its session on Joel's keys; parked, none runs.
    function asking(session) {
        return SessionRow.asks(session.presence) && !Sessions.parkedAs(session.address);
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
    // the right arrow does: Deal With It and Talk where it has a session,
    // Go To first where it's done in a window of its own and after them
    // where not, then Done and Not Now, which stay open as the item leaves
    // its place, then its quest.
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
            const talk = talkTo(WaitingOnYou.session(one));
            return (own ? [does(one), deal(one), talk] : [deal(one), talk, does(one)]).concat([
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

    // What the agents left for Joel to look at: a pull request, a page, a
    // file, a diff or a terminal, with why. The to-do daemon (modules/to-do)
    // watches the harness's looks folder and streams each look whole, and
    // settles one as Joel asks, so nothing here lists, reads or writes a
    // file: one Rust daemon per domain (WCJQSG, 2026-11-09), and one copy
    // of the looks rather than the 421 files the shell read and parsed on
    // Joel's machine (2026-11-13). The stream is ForYouHolds', heard while
    // the shell runs, so an opening draws from what it holds and never
    // waits on it. A machine without the daemon has no looks to show.
    readonly property var lookStream: ForYouHolds.lookStream
    // Each look's text, by its file's name, as the stream last told it,
    // with each Joel settled laid over it until the stream says so too.
    property var lookTexts: ({})
    // Each look is kept by its text, in looksParsed.by, so only a look
    // that changed is parsed again.
    readonly property var looksSplit: Looks.looks(Object.keys(lookTexts)
        .map(file => ({ file: file, text: lookTexts[file] })), looksParsed, looksHour * 3600000)
    // Those read, each one Joel's answer has handled in History (Looks.js's
    // taken), so it leaves To Look At once the node takes the answer.
    readonly property var looks: Looks.taken(looksSplit, lookAnswers,
        label => waitingSaid("answering", { choice: label }))
    readonly property var looksParsed: ({})
    // The hour the looks were last listed in, which a look that has waited
    // three days goes quiet by (Looks.js's quieted): set as the launcher
    // opens and as the stream tells the looks again, it changes at most once an
    // hour, so the looks are split again then and never on each opening or
    // minute.
    property int looksHour: Math.floor(Date.now() / 3600000)

    // The stream's looks are taken a turn after it tells them, so a burst
    // of changes, as a restart's whole answer, is taken once: taken one by
    // one, each look parsed every other again and banded For You afresh,
    // 166 times in each of Joel's openings, 60 to 82% of the 6.5 s his
    // shell's thread was pegged (H6EDLC's profile, 2026-11-10).
    Connections {
        target: provider.lookStream

        function onRevisionChanged() {
            Qt.callLater(provider.looksHeard);
        }
    }

    function looksHeard() {
        looksTold(lookStream?.held ?? {});
    }

    // The looks as `records` tells them, the stream's records by key, with
    // each Joel settled laid over its record until the record says it too
    // or is gone. Told as they stand, they change nothing.
    function looksTold(records) {
        looksHour = Math.floor(Date.now() / 3600000);
        const next = {};
        for (const one of Holding.lookFiles(records))
            next[one.file] = one.text;
        for (const file of Object.keys(looksSettling)) {
            const told = records[`look:${looksSettling[file].id}`]?.look;
            if (told !== undefined && told?.state !== looksSettling[file].state) {
                next[file] = looksSettling[file].text;
                continue;
            }
            if (told !== undefined && Looks.settledAlike(looksSettling[file].text, next[file]))
                looksAsShown[file] = { told: next[file], text: looksSettling[file].text };
            delete looksSettling[file];
        }
        // The daemon's text of a look settled just as the act showed it is
        // the same look, so the act's text stands while the stream tells it
        // so (Looks.settledAlike).
        for (const file of Object.keys(looksAsShown)) {
            if (next[file] === looksAsShown[file].told)
                next[file] = looksAsShown[file].text;
            else
                delete looksAsShown[file];
        }
        const files = Object.keys(next);
        if (files.length === Object.keys(lookTexts).length
            && files.every(file => lookTexts[file] === next[file]))
            return;
        lookTexts = next;
    }

    // Each look Joel settled, by its file's name, `{ id, state, text }`,
    // while the stream still tells it as it was.
    readonly property var looksSettling: ({})
    // Each look the stream told settled as the act showed it, by its
    // file's name, `{ told, text }`: the stream's text, and the act's, which
    // stands for it while the stream tells it so.
    readonly property var looksAsShown: ({})

    // A look Joel is done with, shown so in the act's own frame and asked
    // of the to-do daemon, which writes its file whole (#4012's
    // settle-look). One it doesn't settle, refused or not there to ask, is
    // put back as it read before the act and said in the journal, since a
    // look shown cleared that comes back later would say nothing of why
    // (Y2YBTS, 2026-11-10).
    function settleLook(one, state, how) {
        if (one.file === "")
            return;
        const was = lookTexts[one.file] ?? "";
        const text = Looks.rewritten(one, state, how, Date.now());
        delete looksAsShown[one.file];
        looksSettling[one.file] = { id: one.id, state: state, text: text };
        lookTexts = Object.assign({}, lookTexts, { [one.file]: text });
        lookStream.ask({ ask: "settle-look", look: one.id, state: state, how: how },
            answer => provider.lookSettled(one.file, text, was, answer));
    }

    // Put back only while nothing has settled or told the look since.
    function lookSettled(file, text, was, answer) {
        if (answer?.done === true)
            return;
        console.warn(`Now: the to-do daemon didn't settle ${file} (${answer?.error?.message
            ?? "not there to ask"}), so its look stands as it was`);
        if (looksSettling[file]?.text !== text)
            return;
        delete looksSettling[file];
        if (lookTexts[file] === text && was !== "")
            lookTexts = Object.assign({}, lookTexts, { [file]: was });
    }

    // The session a terminal look is about, by the session or address it
    // names, or null when none runs.
    function lookSession(one) {
        return Sessions.running.find(session => ForYou.leftBy(one, session)) ?? null;
    }

    // Opening a look: a terminal's session gone to, anything else opened
    // by what opens its link or its file. Opened, it no longer needs Joel,
    // so a waiting one is written so and moves to History, as does one
    // gone quiet there, whose file still says it waits.
    function openLook(one) {
        if (one.kind === "terminal") {
            goToCell(one);
        } else {
            Niri.spawn(["xdg-open", one.target]);
        }
        if (one.state === "waiting" || one.state === "quiet")
            settleLook(one, "opened", waitingSaid("openedHow", {}));
    }

    // The cell a look leads to, running or parked, as Looks.js finds it:
    // the session that left it, else its quest's own, else one on its quest.
    function lookCell(one) {
        return Looks.cellOf(one, Sessions.running, Sessions.parked);
    }

    // Goes to the window of the cell a look leads to, as it stands when
    // pressed, waking it first where it's parked, as a want's Go To does.
    function goToCell(one) {
        const cell = lookCell(one);
        if (cell === null)
            return;
        Sessions.wakeToGo(Sessions.parkedAs(cell.address), cell);
    }

    // What a look's row does with the cell behind it (Joel, 2026-11-16:
    // "the look list is useless. i have no way to go from the thing it
    // shows to engaging with the thing working on it"): Talk, Agents' own,
    // whose words go to that cell unless it's asking (talkTo), and Go To
    // its window, which a terminal's Open already is, offered as Agents
    // offers it: where it has a window, or is parked and wakes into one;
    // then Explain It, which says what the cell works on in a sentence or
    // two (explainAct), where the machine has Ask's pi to answer it. None
    // while no cell is held for it, or Agents isn't there to talk through.
    function cellActs(one) {
        const cell = lookCell(one);
        const agents = provider.source("agents");
        if (cell === null || !agents)
            return [];
        const talk = [talkTo(cell)].filter(act => act !== null);
        const explain = source("ask") === null ? []
            : [explainAct(`verb:on:waiting:explain:on:look:${one.id}`, cell)];
        const reachable = agents.windowedOf(cell) || !!Sessions.parkedAs(cell.address);
        return one.kind === "terminal" || !reachable ? talk.concat(explain) : talk.concat([thing({
            // Under Go To's own word, so the : line names it as a want's is.
            key: `verb:on:waiting:go:on:look:${one.id}`,
            verb: Words.verb("verb:on:waiting:go"),
            name: Words.name("verb:on:waiting:go"),
            glyph: Words.glyph("verb:on:waiting:go"),
            hue: "action",
            side: cell.title,
            later: true,
            run: () => provider.goToCell(one)
        })], explain);
    }

    // Explain It on `cell`, under `key`: the machine's quick model asked
    // what the session works on, why and how it stands, its answer then
    // standing in the act's place (Explainer.qml), so Joel needn't open a
    // session to know what a short line is about (WCJQSG, 2026-11-17).
    function explainAct(key, cell) {
        const shown = explainer.shown(key);
        return thing(Object.assign({
            key: key,
            verb: Words.verb("verb:on:waiting:explain"),
            name: Words.name("verb:on:waiting:explain"),
            glyph: Words.glyph("verb:on:waiting:explain"),
            hue: "action",
            side: cell.title,
            stays: true,
            changes: false,
            run: () => explainer.explain(key, cell.address)
        }, shown === null ? {} : {
            name: shown.name,
            reads: shown.reads,
            note: shown.note,
            hue: shown.hue
        }));
    }

    // Explain It's asking and answering, for every row with it.
    Explainer {
        id: explainer

        realm: Sessions.realm
        launcher: provider.launcher
        said: Words.of("verb:on:waiting:explain").said ?? {}
        onChanged: provider.changed()
    }

    // The answers Joel gave looks, `{ [look id]: { label, at, taken } }`,
    // each shown as given while the node is asked (Looks.js's answering),
    // then, once it took it, in History (Looks.js's taken). Replaced
    // whole, so what reads it hears.
    property var lookAnswers: ({})

    // Answers a look with one of its choices where it is, rather than in a
    // session that has moved on (Joel, 2026-11-11: "engaging with drops me
    // into a pi sessino that has already moved on from it is a waste of my
    // time"): a steer tells the session that left it, or its quest's, which
    // writes the look resolved. One the node took leaves what waits at
    // once; one it didn't is no longer shown as given, so its choices
    // come back.
    function answerLook(one, choice) {
        const told = Looks.answer(one, choice, lookSession(one) !== null, Date.now());
        if (told === null)
            return;
        lookAnswers = Object.assign({}, lookAnswers,
            { [one.id]: { label: choice.label, at: Date.now() } });
        Sessions.steer(told.address, told.steer, put => {
            const left = Object.assign({}, provider.lookAnswers);
            if (put)
                left[one.id] = Object.assign({}, left[one.id], { taken: true });
            else
                delete left[one.id];
            provider.lookAnswers = left;
        });
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
        const target = ["pr", "page", "diff"].includes(one.kind) ? one.target : "";
        // Its choices and what happens if Joel does nothing, in place of a
        // weight and when it clears, which told him a score and nothing to
        // do (Joel, 2026-11-19).
        const choices = lookChoices(one, now).map(choice => choice.label);
        const facts = [
            fact("choices", choices.join(" \u00b7 ")),
            fact("ifNothing", waits ? Looks.ifNothingOf(one) || lookLapses(one, say) : ""),
            fact("since", one.since === null ? "" : say.clock(one.since)),
            fact("resolved", waits ? "" : ForYou.historySide(one, say)),
            // What it points at, short; its whole address with its id in
            // the nerdy bits, where y copies it (Y2YBTS, 2026-11-12).
            fact("target", ForYou.pointsAt(one.kind, target, say))
        ].filter(each => each.value !== "");
        return Object.assign(lookFrom(one, say), {
            title: one.title,
            facts: facts,
            nerd: [fact("target", target), fact("look", one.id)],
            body: one.words
        });
    }

    // Who a look is from, its drawer's eyebrow, and how that stands where
    // it isn't active, in place of a Session fact saying it again.
    function lookFrom(one, say) {
        return {
            eyebrow: ForYou.sessionSaid(lookAsker(one), lookQuest(one)?.status),
            eyebrowNames: true
        };
    }

    // Who asks, in plain words (Looks.js's askerOf), else its session's
    // title, never an id: "from Trunkd Disk Saturation During Compaction
    // (INV-31016) \u00b7 to look at" told Joel nothing (2026-11-19).
    function lookAsker(one) {
        const title = one.quest ? WaitingOnYou.titles?.[one.quest] : undefined;
        return Looks.askerOf(one, title === undefined ? null : { title: title })
            || lookSession(one)?.title || "";
    }

    // What becomes of a look its session says nothing of: it expires at its
    // expiresAt, which the harness gives one asking nothing, else it goes
    // quiet three days after it came, as Looks.js's quieted has it.
    function lookLapses(one, say) {
        const expires = ForYou.when(one.expiresAt);
        return expires === null ? say.ifNothing
            : waitingSaid("expires", { clock: say.clock(expires) });
    }

    function lookQuest(one) {
        return one.quest ? WaitingOnYou.ownerOf(one.quest) : null;
    }

    // The choices a look offers while it waits and none is given yet.
    function lookChoices(one, now) {
        return one.state === "waiting" && Looks.answering(one, lookAnswers, now) === ""
            ? one.offers : [];
    }

    // A look's row: its title, who asks in its note, its kind's mark, and
    // at its side what it asks, else the strongest reason it's there, or,
    // in History, how it went and when. Return opens it, or goes to a
    // terminal's session, and never answers: an answer is one Return only
    // once Joel has stepped into what asks (modules/keys/language.toml's
    // answer), and nothing holds a look's writer to putting the safe choice
    // first. Its acts, stepped into with Tab as any row holding rows is
    // (QueryKeys.js's into), hold its choices first, then Open, then Talk
    // and Go To on the cell behind it (cellActs), then Clear It, an ending
    // act d does and Return never does (attachments/what-needs-me-table.md's
    // shape, Y2YBTS's ccbc1a56). Its note says who asks while it waits, and
    // why once in History, as the to-do daemon's History rows say it.
    function lookRow(one, now) {
        const say = forYouSaid(now);
        const waits = one.state === "waiting";
        const choices = lookChoices(one, now);
        const opens = one.kind === "terminal" ? "verb:on:waiting:go" : "verb:on:look:open";
        const answering = choice => ({
            verb: Words.verb("verb:on:look:answer"),
            stays: true,
            run: () => provider.answerLook(one, choice)
        });
        const returns = {
            verb: Words.verb(opens),
            later: true,
            run: () => provider.openLook(one)
        };
        const inside = () => choices.map(choice => thing(Object.assign({
            key: `verb:on:look:answer:${one.id}:${choice.value}`,
            name: choice.label,
            glyph: Words.glyph("verb:on:look:answer"),
            hue: "action"
        }, answering(choice)))).concat([thing({
            key: `verb:on:look:open:${one.id}`,
            verb: Words.verb(opens),
            name: Words.name(opens),
            glyph: Words.glyph(opens),
            hue: "action",
            later: true,
            run: () => provider.openLook(one)
        })], provider.cellActs(one), waits ? [thing(Object.assign({
            key: `verb:on:look:clear:${one.id}`,
            verb: Words.verb("verb:on:look:clear"),
            name: Words.name("verb:on:look:clear"),
            glyph: Words.glyph("verb:on:look:clear"),
            hue: "action",
            stays: true,
            run: () => provider.settleLook(one, "cleared", waitingSaid("clearedHow", {}))
        }, Words.ending("verb:on:look:clear")))] : []);
        return member(Object.assign({
            thing: true,
            key: `on:look:${one.id}`,
            forYou: { kind: "look", source: "looks" },
            name: one.title,
            glyph: Theme.glyph[ForYou.glyphOf(one.kind)],
            hue: waits ? "strong" : "muted",
            side: lookSide(one, now, say),
            note: waits ? lookAsker(one) || one.why : one.why,
            drawer: () => provider.lookDrawer(one),
            verbs: inside,
            targets: inside
        }, returns));
    }

    // What a look's row says at its side: the answer given, while it may
    // yet take, else what it asks while it waits, else the strongest reason
    // it waits, or, in History, how it went and when.
    function lookSide(one, now, say) {
        const given = Looks.answering(one, lookAnswers, now);
        if (given !== "")
            return waitingSaid("answering", { choice: given });
        if (one.state !== "waiting")
            return ForYou.historySide(one, say);
        return Looks.askOf(one) || ForYou.reason(lookThing(one), now, say);
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
        looksWeighed.times += 1;
        return keptAlike(looksKept, one.file,
            [text, lookSide(one, now, forYouSaid(now)), lookAsker(one)], () => lookRow(one, now));
    }

    // The to-dos Joel said were done, while their quest's file
    // still tells of them (ForYou.js's settledItems).
    readonly property var doneItems: ForYou.settledItems(WaitingOnYou.asked,
        WaitingOnYou.settled, each => WaitingOnYou.identity(each))

    // A done item's row in History: its title, how it went and when at its
    // side, and its drawer as it had, with when it was resolved. Return
    // goes where it was done, while that's open.
    // Kept while the item, its side and its note read the same, as
    // History's other done rows are (doneKept).
    function doneRow(one) {
        const each = one.item;
        const say = forYouSaid(Date.now());
        const side = ForYou.historySide({ how: say.doneByYou, resolved: one.resolved,
            since: null }, say);
        const note = noteOf(each);
        return keptAlike(doneKept, `done:${WaitingOnYou.identity(each)}`,
            [JSON.stringify(each), side, note], () => madeDone(each, side, note));
    }
    function madeDone(each, side, note) {
        return member({
            thing: true,
            key: `on:waiting:done:${WaitingOnYou.identity(each)}`,
            verb: Words.verb("verb:on:waiting:go"),
            name: each.title,
            glyph: Theme.glyph.waitsOnJoel,
            hue: "muted",
            side: side,
            note: note,
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
    // Its rows are kept while all they're made of is the same, the notices
    // as PastNotices.qml keeps them, the rest by identity, and the day,
    // since each says when it was done as a moment the day's turning
    // moves. Both verbs and targets ask for them at each step, and making
    // all 594 to 679 of Joel's anew took 43 to 92 ms of steps of 81 to 152
    // (2026-11-13). The notices' rows come ready from the notices daemon
    // (row.rs's notices-history) and the looks' from the to-do daemon
    // (doneLooks), each listed as served, and joined as each is ordered
    // rather than sorted again: making Joel's 470 looks done with took
    // 104 to 226 ms of a fresh shell's first step (2026-11-14). Joining
    // them with the rest is a stopgap until For You's rows are served
    // whole.
    readonly property var historyKept: keeper()
    // Each done row History holds, kept while what it's made of is the
    // same, since History is made again at any change it hears, a notice's
    // or a to-do's, and each made every done row anew: 10 to-dos' rows
    // about every 4 minutes on Joel's shell with the launcher shut
    // (RCTRF8, 2026-11-15). Made here only until the notices daemon
    // serves them, as it serves the notices' and the looks' (row.rs).
    readonly property var doneKept: keeper()
    function historyRows(from) {
        const now = Date.now();
        const served = source("notices")?.pastRows(pastOf(from)) ?? [];
        const ours = from === undefined;
        const looked = ours ? lookedRows() : null;
        return keptAlike(historyKept, from ?? "", [served, looked, cameDone,
            ours ? looks.history : null, ours ? doneItems : null, ours ? given.done : null,
            new Date(now).toDateString()], () => ours ? madeWhole(served, looked, now)
            : madeHistory(from, served, looked, now));
    }
    // The whole History, keeping only the done rows it still holds.
    function madeWhole(served, looked, now) {
        keepingBegun(doneKept);
        const made = madeHistory(undefined, served, looked, now);
        keptOnly(doneKept);
        return made;
    }
    function madeHistory(from, served, looked, now) {
        const came = from === undefined ? cameDone
            : cameDone.filter(one => one.source === from);
        const say = forYouSaid(now);
        // The looks done with that the shell holds and the to-do daemon
        // doesn't serve: one answered while its session hasn't written it
        // so (Looks.js's taken), or settled while the daemon hasn't told it
        // yet, and every one while the daemon serves none. Few, they're made
        // here and sorted with what else the shell holds.
        const apart = from === undefined ? lookedApart(looked) : [];
        const ours = from === undefined ? apart.map(one => ({
            at: one.resolved ?? one.since, row: () => provider.keptLook(one, now) }))
            .concat(doneItems.map(one => ({ at: one.resolved, row: () => provider.doneRow(one) })),
                given.done.map(one => ({ at: one.done.at,
                    row: () => provider.givenDone(one, say) })))
            : [];
        const own = Order.sorted(ours.concat(came.map(one => ({ at: one.at,
            row: () => provider.cameDoneRow(one) }))), Order.stream(one => one.at))
            .map(one => ({ at: one.at, row: one.row() }));
        const done = list => list.map(row => ({ at: row.doneAt, row: row }));
        return Order.merged([done(looked ?? []), done(served), own],
            Order.stream(one => one.at)).map(one => one.row);
    }
    // A served look as the shell holds it, by its id, since the daemon's
    // names no file, or as the daemon read it where the shell holds none.
    function lookHeld(held) {
        return looks.history.find(each => each.id === held.id)
            ?? Object.assign({}, held, { file: "" });
    }
    // Opening a served look: as the shell opens its own, and one the shell
    // holds no file of, still waiting or gone quiet, settled by the daemon
    // by its id (FEH1HE, 2026-11-14).
    function openServed(one) {
        openLook(one);
        if (one.file === "" && (one.state === "waiting" || one.state === "quiet"))
            lookStream.ask({ ask: "settle-look", look: one.id, state: "opened",
                how: waitingSaid("openedHow", {}) }, () => {});
    }
    // The looks done with in the shell's own split that doneLooks doesn't
    // serve, by their ids.
    function lookedApart(looked) {
        const served = new Set(looked.map(row => row.look));
        return looks.history.filter(one => !served.has(one.id));
    }
    // How many times a look's side has been worked out, which a step
    // that changed no look adds nothing to.
    readonly property var looksWeighed: ({ times: 0 })

    // The looks done with, each its History row as the to-do daemon
    // serves it ready (FEH1HE, 2026-11-14), newest done with first, held
    // over a connection of their own whether or not the launcher's open,
    // so a fresh shell's first step into History finds them made. Only
    // what the shell alone knows is its own: who a look is from, as the
    // index names its quest, and going to a terminal's session.
    ReadyRows {
        id: doneLooks
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/to-do.sock`
        place: "looks-history"
        beside: ["summary"]
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => doneLooks.thing(row, key, provider.lookServed(row, key))
    }
    // One list of the served rows while their places stand, as the
    // daemon's rows() gives a new list each time.
    readonly property var lookedPage: ({ at: -1, rows: [] })
    function lookedRows() {
        if (lookedPage.at !== doneLooks.placings) {
            lookedPage.rows = doneLooks.rows();
            lookedPage.at = doneLooks.placings;
        }
        return lookedPage.rows;
    }
    // History's rows made while the launcher is shut, so a step into it
    // finds them made: on Now's turn as the shell starts (Launcher.qml's
    // warmNext), and once the to-do daemon or the notices tell a change,
    // after the turn, since the daemon's first answer may come later. A
    // served row is made only as it's listed, so a fresh shell's first
    // step still made Joel's 564 and took 137 ms, 80 of them making
    // (his journal, 2026-11-14 16:29). Open, a change waits for the step.
    onWarmingChanged: {
        if (warming)
            historyRows();
    }
    function historyAhead() {
        if (!(launcher?.open ?? false))
            Qt.callLater(provider.historyMadeAhead);
    }
    function historyMadeAhead() {
        if (!(launcher?.open ?? false))
            historyRows();
    }
    Connections {
        target: doneLooks

        function onHeard() {
            provider.historyAhead();
        }
    }
    Connections {
        target: provider.source("notices")

        function onChanged() {
            provider.historyAhead();
        }
    }
    // A served look's row, its key the one the shell's own gave it, so
    // the launcher keeps its place: Return opens it as the shell's own
    // does, the look as the shell holds it, so one gone quiet is written
    // opened, and its drawer says who it's from.
    function lookServed(row, key) {
        const held = row.held ?? {};
        const one = () => provider.lookHeld(held);
        const opens = held.kind === "terminal" ? "verb:on:waiting:go" : "verb:on:look:open";
        const inside = () => [thing({
            key: `verb:on:look:open:${held.id}`,
            verb: Words.verb(opens),
            name: Words.name(opens),
            glyph: Words.glyph(opens),
            hue: "action",
            later: true,
            run: () => provider.openServed(one())
        })];
        return {
            key: `on:look:${held.id}`,
            look: held.id,
            doneAt: held.doneAt ?? 0,
            forYou: row.forYou ?? { kind: "look", source: "looks" },
            verb: Words.verb(opens),
            later: true,
            run: () => provider.openServed(one()),
            verbs: inside,
            targets: inside,
            actsOnly: false,
            drawer: () => Object.assign({}, doneLooks.drawerOf(row, key),
                provider.lookFrom(one(), forYouSaid(Date.now())))
        };
    }
    // What a source gave that's done, in History as its source made it,
    // saying at its side how it went and when, "read 14:02", as every row
    // there does.
    function givenDone(one, say) {
        const side = ForYou.historySide({ how: one.done.how, resolved: one.done.at,
            since: null }, say);
        return keptAlike(doneKept, `given:${one.row.key}`, [one.row, side],
            () => Object.assign({}, one.row, { side: side, later: true }));
    }
    readonly property int resolvedCount: looks.history.length + doneItems.length
        + cameDone.length + given.done.length + pastKept()

    // Whose notices History holds (Lately.js's pastSources), narrowed to
    // `from`, one of them, as a source's own History is.
    readonly property var pastSources: Lately.pastSources(finishedOf("slack"))
    function pastOf(from) {
        return from === undefined ? pastSources : pastSources.filter(one => one === from);
    }
    // How many notices done with History keeps of `from`, or of all it
    // holds, as the notices daemon counts them.
    function pastKept(from) {
        return Lately.pastCount(source("notices")?.pastSummary ?? null, pastOf(from));
    }
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
        const count = cameDone.filter(one => one.source === from).length + pastKept(from);
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
    // through from home: "notices", as Super+N with none on screen and the
    // margin's Show Them ask, is Desktop, one key to one place whatever it
    // holds (VKCF3Z, 2026-11-09), never the Notices place it once was,
    // found where it stands: in For You while a critical notice waits
    // there, else among Now's own rows (latelyInForYou). The notices stay
    // found by `/` (PastNotices.qml). "forYou",
    // as the margin's tray is pressed, is For You itself. "onScreen", as
    // Super+N with notices on screen, is those notices alone (onScreenRow),
    // passing through where Desktop stands, so - climbs to the notices'
    // own place rather than a For You that may hold nothing of them.
    leadsInto: ["forYou", "notices", "onScreen"]
    function leadsTo(name) {
        // Only those it says, so the chords' places, which read the list
        // alone, never disagree with where it leads.
        if (!leadsInto.includes(name))
            return null;
        if (name === "forYou")
            return [waiting];
        const { holding } = latelyBands(Date.now());
        const desktop = keptNoticesRow("desktop", Lately.members(holding.desktop, one => one.who),
            { kind: "notice", source: "desktop" });
        const through = latelyInForYou(desktop) ? [waiting] : [];
        return through.concat(name === "onScreen" ? [onScreenRow()] : [desktop]);
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
    // A thing to try a day old is done as not tried, judged at the looks'
    // hour (looksHour), so it goes within the hour and never on a minute.
    readonly property var given: ForYou.given(provider.launcher?.sources ?? [],
        looksHour * 3600000, (Words.of("on:waiting").said ?? {}).notTried)
    // What the sources give, as Now reads them, counted with the rest where
    // the tray counts them too; and the looks that wait, as Now takes them
    // from the to-do daemon's stream, Joel's own acts laid over it, so the
    // tray and For You say one number and parse the looks once.
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
    // A session's own notices listed under its cell, which the tray counts
    // as For You does, each once (Holding.js's joined).
    Binding {
        target: ForYouHolds
        property: "joined"
        value: provider.cellNotices
    }
    // What waits on Joel now, among it a critical notice unread, which
    // waits in For You in the Lately row holding it, as the tray says
    // (ForYouHolds.qml; latelyInForYou).
    readonly property int waitsNow: ForYouHolds.count
    readonly property var criticalNotices: ForYouHolds.holds.notices
    // The sessions there are to reopen, Agents' own row, drawn the same as
    // there, which For You lists as a thing to do while any are left (Joel
    // said yes to For You as the one place for what wants him, 2026-11-08),
    // and the tray counts as one (ForYouHolds.qml's reopen).
    readonly property var toReopen: source("agents")?.toReopen ?? null
    readonly property string first: toReopen !== null ? toReopen.name
        : sessions.length > 0
        ? `${sessions[0].title} ${Sessions.wanted(sessions[0].presence?.wants?.kind)}`
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
    // with the prompt filled in, naming what it asked, then Talk to it,
    // Agents' own, once its run has ended blocked (asking, it has none:
    // talkTo), then Go To. One parked is woken by any of them, its window
    // showing it come back, as a key typed there would; Deal With It and
    // Talk wake it once what they say is in its inbox, which it reads as
    // it starts (Sessions.talk).
    // Each is kept while Agents' row for it, what it asked, its title,
    // whether it's parked, its answers and its want's kind are the same,
    // the kind deciding Talk (talkTo), as Agents keeps its own row while
    // its window and how it stands are (wantsKept).
    function wantRow(made, session) {
        // What it asked, in its own words, or the session by its name.
        const asked = String(session.presence?.wants?.said || session.title);
        const resting = Sessions.parkedAs(session.address);
        // Its answers as the agents daemon serves them (wants.rs), the one
        // it recommends first, or none.
        const answers = source("agents")?.wantOf(session.address)?.answers ?? null;
        wantsNow[session.address] = session;
        return keptAlike(wantsKept, session.address,
            [made, asked, resting, session.title, JSON.stringify(answers),
                Sessions.wantKnown(session.address), session.presence?.wants?.kind],
            () => madeWantRow(made, session, asked, resting, answers));
    }
    readonly property var wantsKept: keeper()
    // Each session wanting Joel as For You last read it, which a kept
    // row's acts go to: Go To finds its window, its kitty and where it
    // runs from the session, which can move while its row reads the same
    // (Y2YBTS, 2026-11-10). Plain, so holding it notifies nothing.
    readonly property var wantsNow: ({})

    function madeWantRow(made, madeFrom, asked, resting, answers) {
        if (answers !== null)
            return answerRow(made, madeFrom, asked, resting, answers);
        const key = `on:waiting:want:${madeFrom.address}`;
        const address = madeFrom.address;
        // The ask this row shows, which Put It Off names (wantRow remakes
        // the row when it changes).
        const known = Sessions.wantKnown(address);
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
            talkTo(session()),
            thing(Object.assign(named("go"), {
                key: `verb:on:waiting:go:${key}`,
                run: () => Sessions.wakeToGo(resting, session())
            })),
            putOffWant(address, key, known)
        ].filter(row => row !== null);
        const row = member(Object.assign({}, made, {
            key: key,
            forYou: { kind: "agent", source: "sessions" },
            // The mark it wears not waiting: every session here waits, and
            // the person with a clock is a to-do's here (ForYou.js). Its
            // hue too, so they aren't all alert and a nearing limit warns.
            glyph: made.mark ?? made.glyph,
            hue: made.markHue ?? made.hue,
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

    // Put It Off, for an hour, on what a session asks Joel: the agents
    // daemon keeps it put off and says so, so it leaves For You and rings
    // for nothing until then, and comes back if it still stands (WCJQSG,
    // 2026-11-16, on Joel's "i can't get this highlighted notice to go
    // away" of a yes that waited five hours). It names the ask the row
    // showed (`known`), so the daemon refuses it once the session asks
    // anew. It stays open, as a to-do's Not Now does, as its row leaves.
    function putOffWant(address, key, known) {
        return thing({
            key: `verb:on:waiting:notNow:${key}`,
            verb: Words.verb("verb:on:waiting:notNow"),
            name: Words.name("verb:on:waiting:notNow"),
            glyph: Words.glyph("verb:on:waiting:notNow"),
            hue: "action",
            side: (Words.of("on:waiting").said ?? {}).anHour ?? "",
            gap: true,
            stays: true,
            run: () => Sessions.putOffWant(address, known)
        });
    }

    // A session asking Joel one question with its answers given, answered
    // where he finds it (harness #1394; VKCF3Z's ruling, 2026-11-12): a
    // place, its side saying it needs his answer and how many it offers,
    // its drawer the question and each answer. Inside, each answer is a
    // row, the recommended first and the cursor on it, whose Return
    // answers at once, since stepping in and picking a labelled answer is
    // already deliberate, and y copies its words; then, set apart, Deal
    // With It, and In Your Own Words where it takes them; then Talk, only
    // while it's parked, its ask being open otherwise (talkTo), and Go To
    // the session, which answers nothing. The session
    // says it took the answer by its want leaving, which takes the row
    // out of For You. Its answers are the agents daemon's, which takes the
    // answer too, naming the want itself; the rows are still made here
    // until it serves For You's rows ready, as it does Agents'.
    function answerRow(made, madeFrom, asked, resting, answers) {
        const key = `on:waiting:want:${madeFrom.address}`;
        const address = madeFrom.address;
        const known = Sessions.wantKnown(address);
        const said = Words.of("on:waiting").said ?? {};
        const session = () => provider.wantsNow[address] ?? madeFrom;
        // Said answered only once the node has put it in the session's
        // inbox, since one it couldn't put never reached the session.
        const answer = (fields, label) => {
            const agents = provider.source("agents");
            const answered = put => {
                if (put)
                    provider.launcher?.say([[filledIn(said.answered ?? "", { label }), ""]]);
            };
            if (agents)
                agents.answerWant(address, fields, answered);
        };
        const named = id => ({
            verb: Words.verb(`verb:on:waiting:${id}`),
            name: Words.name(`verb:on:waiting:${id}`),
            glyph: Words.glyph(`verb:on:waiting:${id}`),
            hue: "action"
        });
        const choice = (one, at) => thing(Object.assign(named("answer"), {
            key: `verb:on:waiting:answer:${at}:${key}`,
            name: one.label,
            side: one.recommended ? said.recommended ?? "" : "",
            copies: one.label,
            stays: true,
            // Stepping into what a session asks was the asking, so one
            // Return answers, from the find too, rather than landing first
            // as a change does (VKCF3Z, 2026-11-12: "Return on a choice
            // answers at once").
            sure: true,
            drawer: () => ({
                title: one.label,
                body: one.description || asked,
                facts: [fact("answers", madeFrom.title)].concat(one.recommended
                    ? [fact("recommended", said.recommended ?? "")] : [])
            }),
            run: () => answer({ value: one.value }, one.label)
        }));
        const inside = () => answers.choices.map(choice).concat([
            thing(Object.assign(named("deal"), {
                key: `verb:on:waiting:deal:${key}`,
                gap: true,
                later: true,
                run: () => {
                    // The node wakes one parked once its words are in.
                    Sessions.steer(address, { act: "fill", text: readyToDeal(asked) });
                    Sessions.goTo(session());
                }
            }))
        ], answers.own ? [thing(Object.assign(named("own"), {
            key: `verb:on:waiting:own:${key}`,
            ask: Words.of("verb:on:waiting:own").ask ?? "",
            chip: madeFrom.title,
            given: words => {
                const text = words.trim();
                if (text !== "")
                    answer({ other: text }, text);
            }
        }))] : [], [talkTo(session()), thing(Object.assign(named("go"), {
            key: `verb:on:waiting:go:${key}`,
            side: madeFrom.title,
            later: true,
            run: () => Sessions.wakeToGo(resting, session())
        })), putOffWant(address, key, known)].filter(row => row !== null));
        const count = answers.choices.length;
        const row = member(Object.assign({}, made, {
            key: key,
            forYou: { kind: "agent", source: "sessions" },
            glyph: made.mark ?? made.glyph,
            hue: made.markHue ?? made.hue,
            side: count === 1 ? said.answerOne ?? ""
                : filledIn(said.answer ?? "", { count: String(count) }),
            drawer: () => ({
                title: madeFrom.title,
                facts: [fact("asked", asked)].concat(answers.choices.map(one => ({
                    label: Words.name(`on:waiting:${one.recommended ? "recommended" : "choice"}`),
                    value: one.description ? `${one.label} \u00b7 ${one.description}` : one.label
                })), [fact("session", madeFrom.title)])
            }),
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
    // one, itself; what he can do now; what he's to look at; what the
    // launcher's sources give, by the kind its row says, a to-do where it
    // says none; and each notice unread that names the session which raised
    // it, its row Desktop's, so it stands under that session (What Needs
    // Me, 42P0R4, 2026-11-19), weighed by how long it's waited
    // (cellNotices). A session's row is Agents',
    // so without Agents there is none, and none is counted. Each `cell()`
    // finds the session behind it, or null, read only as its rows are made
    // (cellRows) and its headings heard (cellsTold), since the sessions
    // move far oftener than the items and the bands are kept while the
    // items are the same.
    readonly property var forYouItems: (toReopen === null ? [] : [{
            // Each session to reopen waits on him until he does, so the row
            // blocks as many as there are, and he can do it now: after a
            // boot it's the first thing to do.
            kind: "do",
            thing: { kind: "todo", ready: true, blocks: Sessions.toReopen.length },
            cell: () => null,
            make: () => provider.toReopen
        }]).concat((provider.source("agents") ? sessions : [])
        .map(session => ({
            // Gathered in Agents Asking once two ask (ForYou.js).
            kind: "agent",
            thing: { kind: "todo", blocks: 1, since: session.presence?.wants?.since ?? null },
            cell: () => session,
            make: () => {
                const agents = provider.source("agents");
                const now = provider.wantingNow(session);
                const row = agents ? agents.made(now) : null;
                return row ? provider.wantRow(row, now) : null;
            }
        })).concat(WaitingOnYou.now.map(each => ({
            kind: "do",
            thing: provider.itemThing(each),
            cell: () => WaitingOnYou.session(each),
            make: now => provider.item(each, now)
        })), looks.waiting.map(one => ({
            kind: "look",
            thing: provider.lookThing(one),
            cell: () => provider.lookCell(one),
            // Read here so an answer given relists it, saying so.
            answered: provider.lookAnswers[one.id] ?? null,
            make: now => provider.keptLook(one, now)
        })), looks.reading.map(one => ({
            // A look that asks nothing, To Read's, its row a look's as any.
            kind: "read",
            thing: provider.lookThing(one),
            cell: () => provider.lookCell(one),
            answered: null,
            make: now => provider.keptLook(one, now)
        })), given.waiting.map(one => ({
            kind: one.row.forYou?.kind ?? "do",
            thing: one.thing,
            cell: () => null,
            make: () => one.row
        })), cellNotices.map(notice => ({
            kind: "notice",
            thing: Cells.noticeThing(notice),
            cell: () => Cells.noticeCell(notice,
                [].concat(Sessions.running ?? [], Sessions.parked ?? [])),
            make: () => noticeRow(notice)
        }))))
    // A session's own notice's row, Desktop's own (PastNotices.qml), saying
    // it's to do from the desktop, as :filter reads every row's (ForYouFilter.js),
    // kept while the daemon's row is the same one.
    function noticeRow(notice) {
        const row = provider.source("notices")?.desktopRows([notice.id])[0] ?? null;
        return row === null ? null : keptAlike(itemsKept, `on:notice:${notice.id}`, [row],
            () => Object.assign({}, row, { forYou: { kind: "do", source: "desktop" } }));
    }
    // How many each band holds, as they stand now (ForYou.js's banded):
    // what needs Joel now, lifted looks and things to try among them, then
    // what To Look At and To Try hold, which For You's row says as a run,
    // "1 to do \u00b7 44 to look at \u00b7 2 to try", and home's Now row says
    // the last two of while nothing needs Joel now (VKCF3Z, 3a92c450).
    // Being bound, they're banded as the items change, and again as time
    // alone lifts one, a deadline coming within the day (liftedAt), so the
    // counts say it then; For You's rows are banded again as it's stepped
    // into a minute on too, for their order.
    readonly property var forYouBands: bandsAt(Math.max(liftedAt, Date.now()))

    // When time alone last lifted a row into what needs Joel now, and when it
    // next will (ForYou.js's liftsAt), with one timer set for then rather
    // than a wake each minute: banded only as its items changed, a look due
    // tomorrow stayed in To Look At, and For You's count left it out, past
    // the hour it was due within the day (2026-11-14). A timer runs no more
    // than a day at once, then sets itself again.
    property real liftedAt: 0
    readonly property var liftsAt: ForYou.liftsAt(forYouItems, Math.max(liftedAt, Date.now()))
    onLiftsAtChanged: armLift()
    function armLift() {
        if (liftsAt === null) {
            lift.stop();
            return;
        }
        lift.interval = Math.min(86400000, Math.max(0, liftsAt - Date.now()));
        lift.restart();
    }
    Timer {
        id: lift

        onTriggered: {
            provider.liftedAt = Date.now();
            provider.armLift();
        }
    }

    // For You's items banded, once for each change of them and each
    // minute: its count, its rows and its filter each banded them afresh,
    // three to five times for each act in it and each step into it, as
    // many milliseconds each as it holds items (2026-11-10). The agents
    // daemon takes the banding over once it serves them banded, with the
    // time each changes (KP0L6A, 2026-11-14).
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
    // tray counts it (ForYouHolds.qml), though the Lately row holding it does;
    // one standing under its cell is a row among the bands, which the tray
    // counts as one too, and never again as critical (Holding.js's joined),
    // so every row in the band is counted, once, and the two never differ.
    readonly property int toDoCount: forYouBands.now.length + forYouBands.agent.length
        + criticalNotices.length
    // The notices standing under the cell that raised them, as For You
    // holds them while it's open (latelyNotices), so one read there stays
    // put; those of a cell wanting Joel, or whose want he put off, are its
    // want's to say (ForYouCells.js's joining), and none while no notices
    // source makes their rows, so none is counted with no row. The wanting as For You
    // reads them (wantingHeard), and the same list while it holds the same
    // notices (ForYouCells.js's alike), so a session working, its want the
    // same, or a notice said anew bands nothing again. A row is made from
    // the notice as it is then.
    readonly property var cellNoticesKept: ({ held: [] })
    readonly property var cellNotices: {
        const joined = provider.source("notices") ? Cells.joining(latelyNotices,
            [].concat(Sessions.running ?? [], Sessions.parked ?? []), wantingHeard,
            { putOff: Sessions.putOff }) : [];
        cellNoticesKept.held = Cells.alike(cellNoticesKept.held, joined);
        return cellNoticesKept.held;
    }
    readonly property int toLookAtCount: forYouBands.look.length
    readonly property int toTryCount: forYouBands.try.length
    // For You's row's side, the bands as a run (ForYou.js's bandsRun).
    readonly property var bandsSaid: ForYou.bandsRun({ now: toDoCount, look: toLookAtCount,
        try: toTryCount }, Words.of("on:waiting").said ?? {})

    // The rows of a band, made now, as they're stepped into.
    function bandRows(band, now) {
        return band.map(one => one.make(now)).filter(row => row !== null);
    }

    // What needs Joel now, under a heading for each cell behind it, the
    // cells by their heaviest row and each one's rows by weight but for
    // what it asks, which leads it as what holds it up, with what
    // no session is behind under one heading of his own (ForYouCells.js's
    // byCell; Joel, 2026-11-16: "one 'what needs me' view grouped by
    // cell"). No heading while all of it is his own, and a group whose rows
    // all went as they were made says nothing. `asked`, Agents Asking's row
    // or none, stands first while headings show, else by `weight`, its
    // heaviest, among the loose rows (ForYouCells.js's laidOut). What a
    // cell asks is its want's row, or a to-do of its own it blocks on, as a
    // session whose want is an action asks it (ForYou.js's blocked).
    function cellRows(band, now, asked, weight) {
        const asks = one => one.kind === "agent" || one.kind === "do" && one.thing.blocks > 0;
        return Cells.laidOut(Cells.byCell(band, one => one.cell(), asks),
            items => bandRows(items, now), cellHeading, { rows: asked, weight });
    }

    // A cell's heading, its name and how it stands as the agents daemon
    // words them for Agents' own row (ForYouCells.js's headingName), and
    // Joel's own heading where no cell is. Kept while it says the same. Its
    // key sits outside on:waiting's, where a to-do's quest could be "cell".
    function cellHeading(cell) {
        const name = cellSaid(cell);
        const key = `on:heading:cell:${cell === null ? "yours"
            : String(cell.address).toUpperCase()}`;
        return keptAlike(cellsKept, key, [name], () => member({ key, name, heading: true }));
    }
    readonly property var cellsKept: keeper()

    function cellSaid(cell) {
        const agents = cell === null ? null : provider.source("agents");
        return Cells.headingName(cell, agents ? agents.standingOf(cell.address) : null,
            Words.of("on:waiting").said ?? {});
    }

    // What the headings over what needs Joel now say, heard with what For
    // You lists from (forYouHeard), so a cell's standing moving asks For
    // You again while only Agents' row for it moved: a cell behind a to-do
    // or a look and wanting nothing went on saying how it stood when For
    // You was made. Words, so it moves only as a heading would read
    // otherwise. It moves as a session turns, working to listening and
    // back, which remakes For You while it's held, as a heading must to say
    // it true; shut, it's only these words made again.
    readonly property string cellsTold: Cells.byCell(forYouBands.now, one => one.cell())
        .map(group => cellSaid(group.cell)).join("\n")

    // A kind's own row, To Look At, To Try or To Read, holding what of that kind
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
    // Agents Asking is such a row too, gathering what agents ask of Joel,
    // though each needs him now, so it stands among what does where its
    // heaviest weighs, its mark in the hue of what it holds rather than
    // muted (Joel, 2026-11-18 10:18: "they are all loose amongst other
    // things properly categorized").
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
            hue: kind === "agent" ? rows[0].hue ?? "" : "muted",
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
    // heaviest first in each: Desktop first while it holds a critical
    // notice (latelyInForYou), since what's pressing leads, so Return,
    // Return reaches a low battery before any question (42P0R4 and
    // Y2YBTS, 2026-11-20); then what needs Joel now, under the cell behind
    // each (cellRows), then To Look At, To Try and To Read, Later and
    // History (VKCF3Z, 2026-11-09, and 2026-11-12 for To Read, which
    // nothing counts).
    // Each asking is a pass of the to-dos', sessions' and kinds' keeping,
    // so what's no longer there is let go; a look is let go once its file
    // is, since History's looks are kept beside them.
    function waitingRows() {
        const now = Date.now();
        const bands = bandsAt(now);
        const keeps = [itemsKept, wantsKept, kindsKept, cellsKept];
        keeps.forEach(keeping => keepingBegun(keeping));
        for (const identity of Object.keys(itemsNow))
            delete itemsNow[identity];
        for (const address of Object.keys(wantsNow))
            delete wantsNow[address];
        const asked = kindRow("agent", bandRows(bands.agent, now));
        const rows = latelyRows(now).filter(latelyComingIn).concat(
            cellRows(bands.now, now, asked, bands.agent[0]?.weight ?? 0),
            kindRow("look", bandRows(bands.look, now)),
            kindRow("try", bandRows(bands.try, now)), kindRow("read", bandRows(bands.read, now)),
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
        return latelyRows(now).filter(latelyComingIn).concat(
            bandRows(bands.now.slice(0, bands.agentAt), now),
            bandRows(bands.agent, now), bandRows(bands.now.slice(bands.agentAt), now),
            bandRows(bands.look, now),
            bandRows(bands.try, now), bandRows(bands.read, now));
    }

    // Whether one of Lately's rows stands in For You: only while its own
    // notices hold a critical one unread, which is one to do (toDoCount),
    // so the row For You counts it in is one For You lists. Mail, Slack
    // and Desktop otherwise ask nothing of Joel, so they stand in Now's
    // own rows, after the moment's, and leave For You to what needs him
    // (Joel, 2026-11-19: "I almost never know what I am supposed to do
    // with it"; Y2YBTS, 2026-11-19). Each asking of `all`, For You and its
    // filter is a pass of all of Lately's rows, so each keeps every row
    // (latelyKept) whichever it shows.
    function latelyInForYou(row) {
        return latelyPlaced[row.forYou?.source] === true;
    }
    // Whether For You's own list holds one of Lately's rows: where it's
    // placed, or one that comes in once Joel is there. The launcher asks
    // a step's rows before it holds the step, so as he steps into For You
    // the row is listed in its place at once, not a refresh later at the
    // end (round 5, F73). Only For You and its filter ask this.
    function latelyComingIn(row) {
        const source = row.forYou?.source;
        return latelyInForYou(row) || latelyCritical?.[source] === true
            && !latelySteps().inside.includes(source);
    }
    // Which sources hold a critical notice unread, each asking its own
    // notices by the tray's rule (Holding.js's held), so a critical one of
    // Slack's lifts Slack's row, not Desktop. They're asked whether or not
    // a source's daemon answers, as the tray counts them, and from the
    // history as it is, not as Now holds it still while open, so a notice
    // Joel read here lifts nothing the tray no longer counts (round 5,
    // F66).
    readonly property var latelyCritical: {
        const up = {};
        for (const source of Lately.SOURCES)
            up[source] = (latelyCriticalHeld[source] ?? []).length > 0;
        return up;
    }
    // Each source's critical notices unread, as latelyCritical asks them,
    // so home names the one leading the row For You leads with.
    readonly property var latelyCriticalHeld: {
        const onScreen = Object.keys(Notices.live).filter(id => !(id in Notices.resting));
        const of = Lately.held(Notices.history, []);
        const up = {};
        for (const source of Lately.SOURCES)
            up[source] = Holding.held({ notices: of[source] ?? [], onScreen: onScreen })
                .notices;
        return up;
    }
    // Where Lately's rows stand. While the launcher is open a row comes
    // into For You when one of its notices turns critical, so what For
    // You counts it lists, but not while Joel stands inside that row or
    // anywhere under Now's own rows but For You, since Now's rows list it,
    // so it never leaves the list he's reading or one - climbs back to
    // (round 4, F51, F61; round 5, F68); it comes in once he steps into
    // For You or out of Now. None
    // leaves until the next opening, so a notice read, withdrawn, woken
    // onto the screen or put away never moves it out from under him. A
    // row left holding nothing still goes (Lately.js's bands).
    // Never bound, since a binding would follow every notice while open.
    property var latelyPlaced: ({})
    onLatelyCriticalChanged: replaceLately()
    onOpenedAtChanged: replaceLately()
    Connections {
        target: provider.launcher
        ignoreUnknownSignals: true

        function onHeldChanged() {
            provider.replaceLately();
        }
    }
    function replaceLately() {
        // Asked as the shell starts too, before latelyCritical is first read.
        const critical = latelyCritical;
        if (critical === undefined)
            return;
        // Open once the launcher is, though Now's opening work waits for it
        // to wake (openingAt), so Desktop keeps its place as it would.
        const shut = openedAt === Number.POSITIVE_INFINITY && openingAt < 0;
        const { inside, underNow } = latelySteps();
        const placed = {};
        for (const source of Lately.SOURCES)
            placed[source] = shut ? critical[source] : latelyPlaced[source] === true
                || (critical[source] === true && !underNow && !inside.includes(source));
        if (Lately.SOURCES.some(source => latelyPlaced[source] !== placed[source]))
            latelyPlaced = placed;
    }
    // Where Joel stands, for Lately's rows: the sources of the rows he's
    // inside, and whether he's under Now's own rows but not in For You,
    // which is the step after Now's own (Provider.qml's heldKeys), or Now's
    // own step itself when he found For You by name from home (round 6,
    // F75).
    function latelySteps() {
        const steps = launcher?.held ?? [];
        const nowAt = steps.findIndex(step => step.row?.from === provider);
        const forYou = step => String(step?.row?.key ?? "").startsWith("on:waitingOnYou");
        return {
            inside: steps.map(step => step.row?.forYou?.source),
            underNow: nowAt >= 0 && !forYou(steps[nowAt]) && !forYou(steps[nowAt + 1])
        };
    }

    // What came for Joel, Lately, in Now's own rows after the moment's, or
    // For You's (latelyInForYou; Joel: "a notice stays until read or
    // forgotten"): a row for each source holding something unread. Mail
    // and Slack are their pieces' rows (NewMail.qml, NewSlack.qml), each
    // piece saying what it holds (Piece.qml's lately
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

    // Desktop and History are Now's, Desktop in For You while it holds a
    // critical notice unread (latelyInForYou), so a notice
    // forgotten, brought back or read is said again in them: the launcher
    // makes a place again only when what it was entered from changes.
    Connections {
        target: Notices

        function onHistoryChanged() {
            provider.noticesMoved("the notices' history");
        }
    }
    // History lists a notice past the notices place's page from the rows
    // asked for once it's stepped into (PastNotices.qml's every), plain
    // until they come, and Desktop lists every notice's row until its own
    // place tells, a frame after the launcher opens, so each is said again
    // whenever the notices' rows change: heard only from History's
    // answers, Desktop's row went on saying "unread" after its own came.
    Connections {
        target: provider.source("notices")

        function onChanged() {
            provider.noticesMoved("the notices' rows");
        }
    }

    // The rows that show the notices themselves: Desktop's, Slack's while
    // its piece can't speak (noticesRow), and what's on screen.
    readonly property var noticesShown: ["on:waiting:desktop", "on:waiting:slack",
        "on:waiting:screen"]

    // The notices moved, as `why` says. While one of Now's rows is held, and
    // nothing below it, it's asked with For You's own, after the change
    // (forYouMoved): said outright there, a session's notice updated as its
    // work turned just after Return into For You was drawn as Return's act,
    // though no row For You showed had moved (the niri case, 2026-11-18:
    // "the notices' history, then the notices' rows"). What such a step
    // shows is rows of places or a thing's acts, whose every drawn line
    // changedTo compares. Where the notices themselves are shown, in
    // Desktop, History or what's on screen (leadsTo), changedTo doesn't
    // reach their rows or compare their hue and drawer, so there, as with
    // nothing held, it's said outright and the places a notice is listed
    // in are made again. Desktop and what's on screen are right under Now
    // while no critical notice waits (latelyInForYou), so it's the step's
    // key that says which, not only its depth (rounds 7 and 8, F77, F80).
    function noticesMoved(why) {
        const held = heldKeys();
        if (held.length === 1 && !noticesShown.includes(held[0])) {
            Qt.callLater(Hearing.putOff(forYouTold));
            return;
        }
        changedWhy = why;
        changed();
        changedWhy = "";
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
        // Desktop's row is asked for whatever it holds (leadsTo), so it's
        // kept with nothing to read too, where its band is left out: let
        // go at each asking, it was made again at each opening into For
        // You with no notice come (2026-11-11).
        const desktop = latelyKept.rows["notices:desktop"];
        if (desktop !== undefined && !("notices:desktop" in latelyKept.next))
            latelyKept.next["notices:desktop"] = desktop;
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
        const done = cameDone.filter(one => one.source === from).length + pastKept(from);
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
        const listed = () => (ids.length > 0 ? (source("notices")?.desktopRows(ids) ?? [])
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
            // The desktop's lists the notices the screen shows, so the arrival
            // line steps aside while it's open (NoticeLine.js's up), each in
            // one place; Slack's, standing in while its daemon doesn't answer,
            // lists Slack's alone, so the rest stay on screen.
            gathers: from === "desktop",
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // The notices on screen, as Super+N lists them, so Joel goes through
    // those the arrival line holds and acts on each where it is (Joel,
    // 2026-11-14: "navigate through the onscreen ones and directly
    // engage, while keeping them a part of the launcher"): those waiting,
    // in the order the line takes them, the one it shows first, as the
    // daemon orders them (front.rs's order). Each is Desktop's row, so its
    // keys, acts and drawer are. One read or come while it's open stays,
    // the new at the end, so no row moves under the cursor: a stopgap
    // merging ids until the notices daemon serves this place's rows whole.
    // No place lists it; it's Super+N's alone, and - climbs to where
    // Desktop stands (leadsTo). The line steps aside while it's open, each
    // in one place.
    function onScreenRow() {
        let ids = (Notices.front.order ?? []).slice();
        const listed = () => {
            const now = Notices.front.order ?? [];
            ids = ids.concat(now.filter(id => !ids.includes(id)));
            return source("notices")?.desktopRows(ids) ?? [];
        };
        const words = Words.of("for-you:source:screen");
        const row = member({
            key: "on:waiting:screen",
            forYou: { kind: "notice", source: "desktop" },
            verb: "more",
            stays: true,
            name: Words.name("for-you:source:screen"),
            glyph: Words.glyph("for-you:source:screen"),
            hue: "muted",
            side: ForYou.filled(words.said?.count ?? "{count}", { count: ids.length }),
            empty: words.empty ?? "",
            nested: true,
            thing: true,
            verbs: listed,
            targets: listed,
            stepped: () => Qt.callLater(() => Notices.seen()),
            gathers: true,
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
    readonly property var cameDone: Lately.done({
        messages: finishedOf("slack"), finished: finishedOf("mail") ?? [] },
        { mail: Words.of("mail:message").said ?? {},
            slack: Words.of("slack:message").said ?? {} })

    // A done one's row in History: the row its piece gives it, as a piece
    // of mail's or a Slack message's, with how and when it was done with
    // at its side.
    function cameDoneRow(one) {
        const side = ForYou.historySide({ how: one.how, resolved: one.at, since: null },
            forYouSaid(Date.now()));
        const own = typeof one.of.row === "function";
        return keptAlike(doneKept, `came:${one.source}:${one.at}`,
            [own ? one.of : null, one.title, side], () => madeCameDone(one, side, own));
    }
    function madeCameDone(one, side, own) {
        const given = own ? one.of.row(thing) : null;
        const row = given ?? member({
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
        Sessions.wanting, cellsTold]
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
    // stepped into, in For You's order as it stands with no heading, which
    // a filter draws none of (ForYou.js's agentAt).
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
    // own, as Lately's rows do.
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
        return standing("all", all).slice();
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
        // The row of Lately's that leads For You (waitingRows) leads home
        // too, by the same test over the same bands, never making its rows,
        // so home names what Return, Return reaches: its critical notice,
        // or the row's own name once that's read or has no summary.
        const leading = latelyBands(Date.now()).bands.map(band => band.source)
            .find(source => latelyComingIn({ forYou: { source: source } })) ?? null;
        const held = leading === null ? [] : latelyCriticalHeld[leading] ?? [];
        const critical = leading === null ? ""
            : String(held[0]?.summary ?? "") || Words.name(`for-you:source:${leading}`);
        const asked = sessions.length > 0 && critical === "";
        const lead = sessions.length === 0 && critical === ""
            ? WaitingOnYou.now[0] ?? null : null;
        // A look or a landing lifted into what needs him now, where no
        // session or to-do leads, says its own row's name.
        const lifted = sessions.length > 0 || lead ? null : forYouBands.now[0] ?? null;
        const title = critical !== "" ? critical : asked ? first : lead ? lead.title
            : lifted?.make(Date.now())?.name ?? "";
        if (title === "") {
            return [];
        }
        const minutes = typeof lead?.minutes === "number"
            ? waitingSaid("minutes", { minutes: lead.minutes }) : "";
        // What it names is one of what needs him now, but for a row whose
        // notice is read while the launcher is open, which nothing counts.
        const more = toDoCount - (critical !== "" && held.length === 0 ? 0 : 1);
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
        // What came for Joel is Lately's, whose rows stand after the
        // moment's or in For You (latelyInForYou), and the notices missed
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
