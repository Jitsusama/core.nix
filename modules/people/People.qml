// People, the launcher's place for everyone Joel deals with, found by
// typing "people" or a name: one person per email address, however many
// programs know them (Person.js joins them), each the same row wherever
// it shows. The programs are its sources (PersonSource.qml), made from
// the files jitsusama.people.sources names, in their order; none is named
// here.
//
// With nothing typed it holds the favourites, then whoever there's been
// something with, the last first (VKCF3Z's design, RSCH-20261028-2NK1E5,
// waiting on Joel). Typing finds anyone any source knows by name or
// address, those listed first. Each says how they stand at the right,
// when a source says, and their address below. Return steps in, as the
// right arrow does, never calling by itself: inside, every way this
// machine reaches them, in the sources' order (calling before mail before
// chat), their page in each source that keeps one, what waits between
// them, their next meeting, when they last met, what they last wrote each
// other, whether they're a favourite,
// then each address to copy, last as every Copy is. Each thing is drawn
// by its own row, so a mail is Mail's row and a meeting the meeting's.
// Their details, every address and each source's facts, are in the
// drawer beside the row, which the daemon serves ready for anyone it
// lists or finds (drawer.rs).
//
// A source the people daemon holds (`daemonHolds`, as Tuple's and Google's) is read from
// the daemon, never held whole here: only those it lists with nothing
// typed, watched as they change, and those a search finds, asked of the
// daemon as each text is typed. Holding Tuple's 7,731 contacts here made
// 23,276 rows at each opening and held Joel's first key a second
// (2026-10-31, 09:31).
//
// Long because it still merges and ranks the people of the sources the
// daemon doesn't hold yet, Slack's; what works on plain values
// lives apart, in Person.js, Recent.js and Zones.js. It shrinks as the
// daemon takes those over, by Fri 13 Nov.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../Hearing.js" as Hearing
import "../When.js" as When
import "Person.js" as Person
import "Placed.js" as Placed
import "ReadyRows.js" as Ready
import "Recent.js" as Recent
import "Zones.js" as Zones

Provider {
    id: provider

    what: "person"

    collection: Words.collection("people")

    // The sources, made from their files as the shell starts; one that
    // fails to load is said and skipped, so the rest still answer.
    property var sources: []

    Component.onCompleted: {
        const made = [];
        for (const listed of settings.sources ?? []) {
            const file = Qt.createComponent(`people/${listed.name}.qml`);
            if (file.status !== Component.Ready) {
                console.error(`People's ${listed.name} couldn't load: ${file.errorString()}`);
                continue;
            }
            // Given its place, so someone's acts learned later, as Slack's
            // are, make the person Joel is inside again.
            made.push(file.createObject(provider, { settings: listed, place: provider }));
        }
        sources = made;
    }

    // The people daemon, asked what a search finds, its answers given to
    // the search alone. It watches nothing: those listed come whole from
    // the lister below.
    Daemon {
        id: book

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/people.sock`
        kind: "person"
        live: false
    }

    // Those listed with nothing typed, kept ready by the people daemon
    // (its listing kind): each one's row in the form every daemon's rows
    // take (ready-rows.md), and their record, as one person with how they
    // stand and what every source told of them, told whole once and after
    // only what changed. So the shell neither merges nor orders
    // them: with 600 people in Qt's engine, merging everyone took the GUI
    // thread 26 to 38 ms, and ordering those listed 19 more (2026-11-10).
    // It's said when the daemon stops answering, and those it last listed
    // stay listed until it answers again.
    Daemon {
        id: lister

        socket: book.socket
        kind: "listing"
        answersWhole: true
        onStateChanged: {
            if (state === "not answering") {
                console.warn("People: the people daemon isn't answering, "
                    + "so those listed are as it last said");
                quiet.restart();
            } else {
                quiet.stop();
                provider.unanswered = false;
            }
        }
        onHeard: (changed, gone) => provider.nameAll(changed, gone)
    }
    // Who the daemon has named, by key, as their name and addresses, to
    // tell a change to who someone is from a change to how they stand.
    // Plain, so keeping it notifies nothing.
    readonly property var namesHeld: ({})
    // Counts each change to who the daemon names, for a binding asking
    // who an address is (whoIs), so a sender or a guest drawn before
    // People answered is named once it does (Y2YBTS, 2026-11-10); how
    // they stand changes far more often, and isn't counted, so it
    // remakes none of those rows.
    property int namesRevision: 0
    // Each listed person's row as the daemon last told it, by its key, and
    // those keys in the order each row's `order` says, each a change names
    // placed as it comes (Placed.js), never sorted. Plain, so keeping them
    // notifies nothing; listedRevision counts each time they're placed,
    // which is after the lister's revision moves, so nothing reads the
    // order before it's placed.
    readonly property var rowsHeld: ({})
    readonly property var placing: ({ keys: [] })
    property int listedRevision: 0
    function nameAll(changed, gone) {
        const rowsChanged = [];
        const rowsGone = [];
        for (const key of changed) {
            const record = lister.held[key];
            if (record?.kind === "row") {
                rowsHeld[key] = record.row;
                rowsChanged.push(key);
            }
        }
        for (const key of gone) {
            if (key in rowsHeld) {
                delete rowsHeld[key];
                rowsGone.push(key);
            }
        }
        placing.keys = Placed.placed(placing.keys, rowsHeld, rowsChanged, rowsGone);
        listedRevision += 1;
        let renamed = false;
        for (const key of changed) {
            const person = lister.held[key]?.person;
            if (!person)
                continue;
            const named = JSON.stringify([person.name, person.emails]);
            if (namesHeld[key] !== named) {
                namesHeld[key] = named;
                renamed = true;
            }
        }
        for (const key of gone) {
            if (key in namesHeld) {
                delete namesHeld[key];
                renamed = true;
            }
        }
        if (renamed)
            namesRevision += 1;
        // Who the daemon told whoIs of is asked again once anything's
        // heard, since what told the listing anew may have told them too,
        // and so is what was typed.
        unlisted.by = {};
        seekAgain();
    }
    // Whether the people daemon has been silent a second, so home says so:
    // a restart or a reconnect answers within it, and saying so at once
    // would blink the count away and back (VKCF3Z, 2026-11-10).
    property bool unanswered: false
    // What the head says of People while it's entered, as home's row says
    // it: the same words in both places, ahead of a find's count, and every
    // person's row as last listed, since a row that isn't someone would
    // throw off the rows' numbers (VKCF3Z, 2026-11-10).
    readonly property string pressing: unanswered ? Words.of("people").said.silent : ""
    Timer {
        id: quiet

        interval: 1000
        onTriggered: provider.unanswered = lister.state === "not answering"
    }
    // Counts each change the daemon says, for a source that keeps
    // something of its own until then (Tuple's favourites changed here).
    readonly property int revision: lister.revision

    // The names of the sources the daemon holds.
    readonly property var heldBy: sources.filter(source => source.daemonHolds)
        .map(source => source.settings.name)

    // What the daemon's search last found, for the words asked, best
    // first; and the words asked last, so each text is asked once.
    property var sought: ({ words: "", records: [] })
    property string seeking: ""
    // Those the search found, by key, for the rows they came with: a listed
    // record's row says how they stand, which one found among everyone
    // doesn't.
    readonly property var soughtOf: {
        const by = {};
        for (const record of sought.records)
            by[record.key] = record;
        return by;
    }
    // As many as a search is answered with, as the daemon does unasked.
    readonly property int soughtMost: 50

    // Everything the people are made from: the daemon's records, and each
    // source's people and how they stand. A change to any marks what's
    // made of them stale, to be made again only once something asks, so
    // a change nobody looks at merges no one: merging everyone took 26 to
    // 38 ms of the GUI thread at every change to any source's people,
    // open or shut, 600 of them in Qt's engine (2026-11-10).
    readonly property var heard: [listedRevision, sought].concat(sources.map(source =>
        [source.people, source.stands, source.standing]))
    onHeardChanged: {
        merging.stale = true;
        rowsOfEveryone.stale = true;
        heardRevision += 1;
        Qt.callLater(Hearing.putOff(lentTold));
    }
    // Timed from the earliest line whose change it tells (Hearing.js).
    readonly property var lentTold: Hearing.carried(() => provider.lentMoved())

    // A row People lends another place (personOf), as a mail's sender or a
    // meeting's guest, says how they stood as it was made, and stayed so
    // while shown (Y2YBTS, 2026-11-10). So while one shows, each place held
    // above it is told its rows changed as anyone's stand might have, and
    // asks for them again, the launcher pacing it; with none shown, or the
    // launcher shut, nothing is told and nothing made.
    function lentMoved() {
        lentStands += 1;
        const launcher = provider.launcher;
        const steps = launcher?.open ? launcher.held ?? [] : [];
        const lent = step => step.row.from !== provider && (step.rows ?? [])
            .some(row => row.component === "person" && row.person !== undefined);
        if (steps.some(lent))
            heldAboveChanged();
    }
    // Tells each place held above People's that its rows changed, so it
    // asks for them again, the launcher pacing it; with the launcher shut,
    // nothing.
    function heldAboveChanged() {
        const launcher = provider.launcher;
        const steps = launcher?.open ? launcher.held ?? [] : [];
        const holders = steps.map(step => step.row.from)
            .filter((from, at, all) => !!from && from !== provider && all.indexOf(from) === at);
        for (const holder of holders)
            holder.changed();
    }
    // Counts each change heard, for a binding that reads what's made.
    property int heardRevision: 0
    // What's merged, and how many times everyone was, for a test to
    // bound: kept in a plain object, since a property written as `all`
    // is made would be read by it too, and loop.
    readonly property var merging: ({ stale: true, merges: 0 })

    // Everyone the sources know as one person each, how they stand, and
    // each record read and each person by address, made again only when
    // asked after a change. Plain, so keeping it notifies nothing.
    function merged() {
        const kept = merging;
        if (!kept.stale)
            return kept;
        kept.records = readRecords();
        kept.recordOf = {};
        for (const record of kept.records)
            kept.recordOf[record.key] = record;
        const daemon = fromDaemon(kept.records);
        kept.persons = Person.merged(sources.map(source => ({
            name: source.settings.name,
            people: source.daemonHolds ? daemon.people[source.settings.name] : source.people
        })));
        kept.stands = sources.map(source => (source.daemonHolds
            ? daemon.stands[source.settings.name] : source.stands));
        kept.byEmail = {};
        for (const person of kept.persons) {
            for (const email of person.emails)
                kept.byEmail[email.toLowerCase()] = person;
        }
        kept.stale = false;
        kept.merges += 1;
        Report.say(`people holds ${kept.persons.length}`);
        return kept;
    }

    // Every record read from the daemon, once each: the latest search's
    // first, in its order, then those listed, the listing's order aside.
    function readRecords() {
        const by = {};
        const all = [];
        for (const record of sought.records.concat(Object.values(lister.held))) {
            if (record.gone || !record.sources || record.key in by)
                continue;
            by[record.key] = record;
            all.push(record);
        }
        return all;
    }

    // The record read for a key, found without making anyone, for a
    // source keeping something of its own until the daemon says it
    // (Tuple's favourites changed here).
    function recordFor(key) {
        return lister.held[key] ?? sought.records.find(record => record.key === key
            && !record.gone) ?? null;
    }

    // A held source's people and stands, from the records read, as a
    // source of its own gives them.
    function fromDaemon(records) {
        const people = {};
        const stands = {};
        for (const source of sources.filter(each => each.daemonHolds)) {
            const name = source.settings.name;
            people[name] = [];
            stands[name] = {};
            for (const record of records) {
                const entry = record.sources?.[name];
                if (!entry)
                    continue;
                // The daemon keeps someone by their address, in the key,
                // or by the source's id for one who has none.
                const email = Person.address(entry.email || addressOf(record.key));
                const id = String(entry.id ?? "");
                const recent = entry.recent === true;
                // When each thing with them was, as Google's say, so one
                // listed here is listed by it (listedHere).
                const spans = entry.spans ?? [];
                people[name].push(email !== ""
                    ? { emails: [email], name: entry.name ?? "", recent: recent, spans: spans }
                    : { emails: [], id: id, name: entry.name ?? "", recent: recent,
                        spans: spans });
                stands[name][email !== "" ? email : `${name}:${id}`] = source.stood(entry);
            }
        }
        return { people: people, stands: stands };
    }

    // The address a record's key holds, or "" for one kept by a source's
    // id, as `person:tuple:108` is.
    function addressOf(key) {
        const kept = String(key).slice("person:".length);
        return kept.includes("@") ? kept : "";
    }

    // What a held source told the daemon of someone, or null.
    function entryOf(person, name) {
        const keys = person.emails.length > 0 ? person.emails.map(email => `person:${email}`)
            : [`person:${person.key}`];
        for (const key of keys) {
            const entry = recordFor(key)?.sources?.[name];
            if (entry)
                return Object.assign({ email: addressOf(key) }, entry);
        }
        return null;
    }

    // Asks the daemon for whoever the words find, once a text; the answer
    // comes within milliseconds and is searched as the rest are, so a
    // search never waits on it. With the daemon gone the text is asked
    // again once it's back.
    // Asked `again`, the answer is searched again only when it finds
    // others than it found before.
    function seek(typed, again) {
        const words = String(typed).trim().toLowerCase();
        if (heldBy.length === 0 || words === "" || words === seeking)
            return;
        seeking = words;
        seekAt = Date.now();
        book.find({ ask: "search", kind: "person", words: words, limit: soughtMost }, answer => {
            if (seeking !== words)
                return;
            if (answer === null) {
                seeking = "";
                return;
            }
            const before = sought.words === words ? keysOf(sought.records) : null;
            sought = { words: words, records: answer.records ?? [] };
            if (again !== true || keysOf(sought.records) !== before)
                foundMore(Date.now() - seekAt < freshFor);
        });
    }
    function keysOf(records) {
        return records.map(record => record.key).join("\n");
    }
    // The words last asked, asked again as the daemon hears anything while
    // the launcher is open, so someone it comes to know after they were
    // typed is found without typing again: Google's people, which came 14
    // s after a name was typed once google-people's look for Google's
    // daemon had backed off, never showed until the words changed
    // (80ECO2, 2026-11-11, vm-desktop-looks-plates). A stopgap until the
    // people daemon ranks everyone itself and streams a held search's
    // answer as it changes, by Fri 13 Nov 18:00.
    function seekAgain() {
        if (seeking === "" || !provider.launcher?.open)
            return;
        const words = seeking;
        seeking = "";
        seek(words, true);
    }
    property real seekAt: 0
    // How soon an answer is ranked as the key's own search: it most often
    // comes in 2 ms, before the key's first frame (427IH3, 2026-10-31).
    readonly property int freshFor: 50

    // Everyone shown, one person each: those the sources give, and those
    // read from the daemon. How many is said as they're merged, for a
    // test to bound.
    function persons() {
        return merged().persons;
    }

    // And each as a row, by key, its name and addresses made ready to
    // match only when first searched, made only when a search looks
    // through them, never for a change nobody searches: a crowd said anew
    // made every row again each time, the launcher open or not, 813 to
    // 2,595 of them an opening on Joel's shell (2026-11-08). Each is kept
    // while the person reads the same. Plain, so keeping notifies nothing.
    readonly property var rowsOfEveryone: ({ rows: [], stale: true, keeping: keeper() })
    function everyone() {
        const made = rowsOfEveryone;
        if (made.stale) {
            keepingBegun(made.keeping);
            made.rows = persons().map(person => {
                // One only the daemon's sources know comes with the row a
                // search finds them as, and its drawer, kept while their record says the
                // same, which the daemon stamps `heard` anew when a source
                // says anything new of them, so each search's answer remakes
                // none it found before; and let go once no answer holds it.
                // The rest are merged here from what QML sources hold, until
                // the daemon holds them too: Slack's people, whose facts
                // slack-people serves (#3045), its source in QML going by
                // Fri 13 Nov.
                const record = readyFor(person);
                return record ? keptOr(made.keeping, person.key, record, () => provider.made(
                    person, Person.standing(person, []), record.row, () => record.drawer),
                    heardAlike)
                    : keptOr(made.keeping, person.key, JSON.stringify(person),
                        () => provider.made(person, Person.standing(person, [])));
            });
            keptOnly(made.keeping);
            made.stale = false;
        }
        return made.rows;
    }

    // Whether two of the daemon's records say the same of someone.
    function heardAlike(was, now) {
        return was?.key === now.key && was.heard === now.heard;
    }

    // The daemon's record for one only its sources know, when it came
    // with their row, or null.
    function readyFor(person) {
        const record = soughtOf[`person:${person.key}`];
        const ready = record?.row && !record.gone;
        return ready && person.from.every(name => heldBy.includes(name)) ? record : null;
    }

    // How each stands, which changes often, read from every source in
    // order, so the first that says is believed.
    function standOf(person) {
        return Person.standing(person, merged().stands);
    }

    // Those listed with nothing typed, as they stand now, the last thing
    // with each measured as they're listed rather than kept up with the
    // clock. Each is kept while the person and how they stand read the
    // same, as everyone's are, so a change remakes only those it changed;
    // and they're made only while they're wanted (Provider.qml's wanted),
    // or as they're asked for, so a change nobody sees makes none and
    // refreshes nothing. Listing them writes into keptListed, a plain
    // object that's never reassigned, so writing it notifies nothing and
    // can't loop (Provider.qml's keeper).
    readonly property var all: provider.wanted && heardRevision >= 0 ? listedNow() : []
    onAllChanged: {
        if (provider.wanted)
            changed();
    }
    function listedNow() {
        // Listed and standing as when last asked, they're the same rows,
        // with no one's text written again to find so.
        const listed = listedPersons();
        const now = provider.minute;
        if (listedRows.from === listed && listedRows.minute === now)
            return listedRows.rows;
        keepingBegun(keptListed);
        const rows = listed.map(person => {
            // One the daemon listed comes with their row ready, kept while
            // their record is the one it last told, which it tells anew
            // only when they changed: so nothing about them is written out
            // again to find it the same, and nothing of the row is worded
            // here.
            // Their row reads only what their record says, but for its
            // order, which places it and is read no further, so it's kept
            // by the record.
            const record = person.stood ? lister.held[`person:${person.key}`] : null;
            const ready = record ? rowsHeld[`row:people:${person.key}`] : null;
            // One saying their time is kept for the minute too.
            if (ready)
                return keptAlike(keptListed, person.key, [record, clocked(ready) ? now : 0],
                    () => made(person, person.stood, ready,
                        () => Ready.drawerServed(ready, lister.held)));
            const stood = person.stood ?? standOf(person);
            return keptOr(keptListed, person.key, JSON.stringify([person, stood]),
                () => made(person, stood));
        });
        keptOnly(keptListed);
        // A minute that moved no one's time leaves the rows as they were,
        // so it tells the launcher nothing.
        const same = listedRows.from === listed && rows.length === listedRows.rows.length
            && rows.every((row, at) => row === listedRows.rows[at]);
        listedRows.from = listed;
        listedRows.minute = now;
        if (same)
            return listedRows.rows;
        listedRows.rows = rows;
        return rows;
    }
    readonly property var keptListed: keeper()
    readonly property var listedRows: ({ from: null, minute: 0, rows: [] })

    // A drawer the daemon served, as the drawer draws one: its times said
    // as of now, a zone as the time there (`there`), and a fact that says
    // nothing, as the time in a zone tzdata doesn't know, left out.
    function drawnFrom(served) {
        const words = Object.assign({ zone: zone => provider.there(zone) },
            Words.field.times ?? {});
        const drawn = Ready.drawerOf(served, Date.now(), words);
        if (drawn)
            drawn.facts = drawn.facts.filter(fact => fact.heading !== undefined
                || fact.value !== "");
        return drawn;
    }

    // The listed rows by key, so a search finds them as they stand.
    readonly property var known: {
        const by = {};
        for (const row of all)
            by[row.key] = row;
        return by;
    }

    // A person's row: their name, how they stand, their first address,
    // and Return the first way a source reaches them, asked as it's used,
    // so a row made for a search costs no source anything until then.
    // One the daemon listed or found is drawn as its ready row says
    // (`ready`), its drawer as the daemon serves it (`served`, read as
    // it's shown).
    function made(person, stood, ready, served) {
        const row = member({
            // A ready row's key is the daemon's; the person it is, which
            // Joel's picks remember, is its `thing`.
            key: ready?.thing ?? `person:${person.key}`,
            component: "person",
            name: ready?.name ?? person.name,
            person: person,
            glyph: ready ? Theme.glyph[ready.glyph]
                : stood.favourite ? Theme.glyph.favourite : Theme.glyph.person,
            hue: ready?.hue ?? "person",
            // A ready row's side is its items (ready-rows.md), said here as
            // one text until ReadyRows.qml draws items (Wed 11 Nov), their
            // zone as their time where it's apart from his.
            side: ready ? Ready.sideSaid(ready.side, Date.now(), rowWords) : stood.said,
            note: ready?.note ?? (person.name === person.email ? "" : person.email),
            nested: true,
            later: true,
            verb: "more",
            stays: true,
            // A person's insides ask every source and are found only from
            // inside them, so Ctrl+]'s look for where a row lives never
            // makes them (Catalogue.js's followsInto): the walk that kept
            // where every row lived made 18,693 of People's 19,461 rows at
            // home in Joel's morning (2026-11-09).
            parted: true,
            run: () => provider.launcher?.enter(row),
            verbs: () => provider.inside(person),
            targets: () => provider.inside(person)
        });
        // Their addresses, made ready to match when first searched, as the
        // name is, and kept while they read the same (Fuzzy.kept): everyone
        // is made again whenever a source knows anyone anew, as each mail
        // and meeting does, and making every address ready again was most
        // of a first search's time, 408 ms of "ma" (2026-10-30), since
        // nearly every address holds both letters.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(person.emails.join(" ")))
        });
        // Their details, made when first shown rather than for everyone each
        // time everyone is made. A source's answer that comes later makes
        // everyone again, and so this row, which reads it then. A row is
        // kept while the person reads the same, so the time where they are
        // is made again once its minute has passed. A function the drawer
        // calls (drawerOf) rather than a getter, which a copy of the row
        // would read, merging everyone for their facts (Y2YBTS, 2026-11-10).
        // One the daemon serves merges nobody; until it serves one, as
        // before its listing comes, it's what this person holds.
        let details = null;
        let detailsAt = -1;
        row.drawer = () => {
            const minute = Math.floor(Date.now() / 60000);
            if (details === null || detailsAt !== minute) {
                const drawn = served ? drawnFrom(served()) : null;
                details = drawn ?? Person.details(person, zone => provider.there(zone));
                detailsAt = minute;
            }
            return details;
        };
        return Object.assign(row, {
            named: Fuzzy.letters([person.name.toLowerCase()]),
            mailed: Fuzzy.letters([person.emails.join(" ")])
        });
    }

    // tzdata's zones, as Clocks reads them (Zones.js), read once when a
    // person's time zone is first shown, and never watched, since the list
    // is built with the shell.
    property var zones: null

    // The time in a zone by tzdata's name, as now, and how far it is from
    // here: "14:05, 5 h ahead", or "" for a zone the list doesn't know.
    function there(id) {
        if (zones === null)
            zones = settings.zones ? Zones.prepare(read(zonesFile)) : { zones: {}, named: {} };
        const zone = zones.zones[id];
        if (!zone)
            return "";
        const now = Date.now();
        const [minutes] = Zones.standing(zone, Math.floor(now / 1000));
        const here = -new Date(now).getTimezoneOffset();
        return `${Zones.clock(new Date(now + minutes * 60000))}, ${Zones.apart(minutes - here)}`;
    }

    // Someone's time at their row's side, "London · 17:02", or "" where
    // it's his own or the list doesn't know their zone (Person.js's
    // awaySide).
    function away(id) {
        if (zones === null)
            zones = settings.zones ? Zones.prepare(read(zonesFile)) : { zones: {}, named: {} };
        const zone = zones.zones[id];
        if (!zone)
            return "";
        const now = Date.now();
        const [minutes] = Zones.standing(zone, Math.floor(now / 1000));
        const here = -new Date(now).getTimezoneOffset();
        return Person.awaySide(id, minutes - here, Zones.clock(new Date(now + minutes * 60000)),
            Words.name("people:away"));
    }

    // The words a row's side is said in: the launcher's times, and a zone
    // as `away` says it.
    readonly property var rowWords: Object.assign({ zone: zone => provider.away(zone) },
        Words.field.times ?? {})

    // Whether a ready row says their time, so it's made again as the
    // minute turns.
    function clocked(ready) {
        return (ready?.side ?? []).some(item => item?.as === "zone");
    }

    // The minute the listed rows were said in, moved on while they're
    // wanted, so those saying someone's time are made again (`clocked`);
    // while they aren't, nothing ticks and nothing is made.
    property int minute: Math.floor(Date.now() / 60000)
    Timer {
        running: provider.wanted
        repeat: true
        interval: 60000 - Date.now() % 60000
        onTriggered: {
            provider.minute = Math.floor(Date.now() / 60000);
            interval = 60000 - Date.now() % 60000;
        }
        onRunningChanged: {
            if (!running)
                return;
            interval = 60000 - Date.now() % 60000;
            provider.minute = Math.floor(Date.now() / 60000);
        }
    }

    FileView {
        id: zonesFile
        path: provider.settings.zones ?? ""
        blockLoading: true
        printErrors: false
    }

    // Every source's acts for them, in the one order.
    function acts(person) {
        return Person.acts(sources.map(source => source.acts(person, provider)));
    }

    // What the right arrow lists: what waits between them first, since it
    // needs him (VKCF3Z, 17e0f150), then every way to reach them, their
    // pages, their next meeting, when they last met and what they last
    // wrote each other (VKCF3Z, 0cea8a93), whether they're a favourite,
    // then each address to copy, last as every Copy is. An address wears
    // its mark, though it's a figure, so the page's column of marks never
    // breaks on its last row (VKCF3Z, 2026-11-08).
    function inside(person) {
        const addresses = person.emails.map((email, at) => copyRow({
            key: `fact:person:email:${at}`,
            name: email,
            side: person.emails.length === 1 ? "their email" : "one of their emails",
            glyph: Theme.glyph.email,
            marked: true
        }));
        const had = between(person);
        return had.waiting.concat(acts(person), pages(person), had.since, favourite(person),
            addresses);
    }

    // Their page in each source that keeps one and knows them, in the
    // sources' order, each Open in that source: a look that leaves them as
    // they are, so after the ways to reach them and before what's with
    // them (VKCF3Z, a91b2722). It opens in the browser, closing the
    // launcher. While a source is still finding out, its row stands in
    // place, dimmed, saying so, and lights there once it answers.
    function pages(person) {
        const looking = Words.of("verb:person:page").said?.looking ?? "";
        return sources.map(source => {
            // Asked first, as asking for the page is what starts a look-up.
            const link = source.page(person);
            return { source: source, link: link, looking: link === "" && source.looking(person) };
        }).filter(each => (each.link !== "" || each.looking) && each.source.called !== "")
            .map(each => thing(Object.assign({
                key: `verb:person:page:${each.source.settings.name}`,
                name: Words.name("verb:person:page", "", { source: each.source.called }),
                verb: Words.verb("verb:person:page"),
                glyph: Words.glyph("verb:person:page"),
                hue: "action",
                later: true,
                run: () => Niri.spawn(["xdg-open", each.link])
            }, !each.looking ? {} : {
                // Dimmed, so Return does nothing until it lights.
                gone: true,
                side: looking.replace("{source}", each.source.called),
                run: () => {}
            })));
    }

    // Whether they're a favourite, a switch, from the first source that
    // keeps favourites and knows them.
    function favourite(person) {
        const kept = sources.map(source => source.favourite(person, provider)).find(Boolean);
        return kept ? [kept] : [];
    }

    // The person an address is, as their one row, for a thing that names
    // them, as a mail's sender or a meeting's guests (Mail.qml,
    // Calendar.qml): null when no source knows them, so nothing is listed
    // that can't step in. Asked of any provider holding `personOf`, so a
    // machine without People lists no one.
    function personOf(email) {
        const person = whoIs(email);
        return person ? made(person, standOf(person)) : null;
    }

    // The person a row another place's daemon serves stands for, as a
    // meeting's guest or a mail's sender google serves with `"lends":
    // "person:<email>"` (ReadyRows.qml): their own row, as personOf makes
    // it, or null for any other thing or anyone People doesn't know, so the
    // served row stands (RTOIMQ and Y2YBTS, 2026-11-12). It's asked as each
    // such row is made, so it only looks up who's held: never a search,
    // and never the daemon, as whoIs asks after an unlisted address.
    function lent(thing) {
        const said = String(thing ?? "");
        if (!said.startsWith("person:"))
            return null;
        const address = said.slice("person:".length).trim().toLowerCase();
        const person = merged().byEmail[address] ?? unlisted.by[address] ?? null;
        return person ? made(person, standOf(person)) : null;
    }
    // Goes up whenever what lent() answers may have changed: who's named, or
    // how anyone stands while a row of theirs is lent (lentMoved), so
    // ReadyRows makes again only its rows that lend.
    readonly property int lentRevision: namesRevision + lentStands
    property int lentStands: 0

    // Who an address is, as People knows them, { key, name, emails, ... },
    // with no row made: for a count of a meeting's guests or a call's name
    // (Calendar.qml, Transcripts.qml), which made each one's whole row; null
    // when no source knows them.
    // It reads namesRevision, so a binding asking follows who's named.
    function whoIs(email) {
        if (namesRevision < 0)
            return null;
        const address = String(email ?? "").trim().toLowerCase();
        return merged().byEmail[address] ?? unlistedOf(address);
    }

    // Those the daemon holds and doesn't list, as a guest seen only at a
    // meeting still to come, by address, as whoIs asked after them: each
    // asked of the daemon once, with every other address asked in the
    // same turn, and null until it answers, when namesRevision says so.
    // One it doesn't hold is asked again the next time, so someone it
    // learns of later is named. Plain, so keeping it notifies nothing.
    readonly property var unlisted: ({ by: {}, due: [] })
    function unlistedOf(address) {
        const kept = unlisted;
        if (address in kept.by)
            return kept.by[address];
        if (heldBy.length === 0 || address === "")
            return null;
        kept.by[address] = null;
        kept.due.push(address);
        if (kept.due.length === 1)
            Qt.callLater(askUnlisted);
        return null;
    }
    function askUnlisted() {
        const kept = unlisted;
        const by = kept.by;
        const due = kept.due;
        kept.due = [];
        // Known asks no source, as a step-in would ask Slack about each.
        book.find({ ask: "known", kind: "person", keys: due.map(one => `person:${one}`) },
            answer => {
                const records = (answer?.records ?? []).filter(record => !record.gone);
                const daemon = fromDaemon(records);
                const persons = Person.merged(heldBy.map(name => ({
                    name: name,
                    people: daemon.people[name]
                })));
                let named = false;
                for (const person of persons) {
                    for (const one of person.emails) {
                        if (by[one] === null && by === kept.by) {
                            by[one] = person;
                            named = true;
                        }
                    }
                }
                for (const one of due) {
                    if (by[one] === null)
                        delete by[one];
                }
                // A meeting stepped into counted its guests as it was made,
                // before the daemon answered, so it's told to count again.
                if (named) {
                    namesRevision += 1;
                    heldAboveChanged();
                }
            });
    }

    // Their next meeting, drawn as a meeting is everywhere, the soonest any
    // source has; a meeting to come isn't one they met at.
    function next(person) {
        const coming = sources.map(source => source.next(person, provider))
            .filter(Boolean).sort((a, b) => a.at - b.at)[0];
        return coming ? [coming.row] : [];
    }

    // What's between them, in place of one Recent, which said when rather
    // than what (Person.between): Waiting, what of theirs waits on Joel
    // and what he waits on from them, how many at its side; their next
    // meeting; Last Met, the meetings and calls they shared, its side when
    // the last was; and Messages, what they wrote each other, its side who
    // wrote last and when. Each stands only while it holds anything, and
    // one thing alone stands as itself (Person.placed). Each thing is its
    // own row, as it's drawn everywhere, once, newest first, nine at most
    // (Recent.js). It's gathered once as they're stepped into, and
    // stepping into one lists what was gathered then. One alone says at
    // its side what its heading would have, "needs you \u00b7 mail \u00b7
    // 14:02" or "met \u00b7 yesterday", since nothing above it says why it's
    // there (VKCF3Z, 17e0f150). Waiting is given apart, as it comes first.
    function between(person) {
        const keyed = (which, list) => list.map((one, at) => ({
            key: one.row?.key || `${which}:${at}`, at: one.at, row: one.row,
            kind: one.kind, mine: one.mine, one: one.one, what: one.what
        }));
        const had = Person.between(
            sources.map((source, which) => keyed(which, source.recent(person, provider))),
            sources.map((source, which) => keyed(which, source.unread(person, provider))),
            Recent.listed);
        const now = Date.now();
        const when = at => When.came(at, now, provider.dates);
        const alone = (key, side) => entry => {
            const said = Words.of(key).said ?? {};
            return Object.assign({}, entry.row, { side: side(entry, when, said) });
        };
        const sided = (key, entries, side) => Person.placed(entries, all => holding(key,
            all.map(one => one.row), side(all[0], when, Words.of(key).said ?? {}),
            Recent.once(all.map(one => one.row.name), {})),
            alone(key, (entry, at, said) => Person.aloneSide(side(entry, at, said), said)));
        return {
            waiting: Person.placed(had.waiting, all => {
                const stands = Recent.said(all.map(one => one.row.name), false, {});
                return holding("person:waiting", all.map(one => one.row), stands.side,
                    stands.held);
            }, alone("person:waiting", Person.waitSide)),
            since: next(person).concat(sided("person:met", had.met, Person.metSide),
                sided("person:messages", had.messages, Person.wroteSide))
        };
    }

    // How People says when, in the vocabulary's words, with the locale's
    // names of days and months, as Mail says it (Mail.qml's dates).
    readonly property var dates: Object.assign({}, Words.of("mail:message").said ?? {}, {
        date: (date, format) => Qt.formatDate(date, format)
    })

    // A row holding some of their things, by the words `key` names. Its
    // rows are both what it holds and what it steps into, as a meeting's
    // Show in Calendar's are, so it steps in at any level: inside People, where a
    // row's insides are its verbs, and inside a person found from the top,
    // where they're its targets.
    // It's a place, stepped into and holding rows, so it says nothing
    // after its name and names what it holds in its drawer, `held`, as a
    // place does (the launcher's `heldNames`; VKCF3Z, dafc2199).
    function holding(key, rows, side, held) {
        const had = () => rows;
        const row = thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "person",
            side: side,
            heldNames: held,
            empty: Words.of(key).empty,
            nested: true,
            stays: true,
            verbs: had,
            targets: had,
            run: () => provider.launcher?.enter(row)
        });
        return row;
    }

    // Inside the place, what's typed finds anyone, as at the top.
    // Ranked here over everyone merged, a stopgap: the people daemon ranks
    // them itself once it holds every source's people, by Fri 13 Nov
    // 18:00, and the shell shows its answer in order.
    function search(typed) {
        return gather(typed);
    }

    // Everyone shown is searched, those the sources give and the daemon's
    // answers, so each name and address's letters are tested first, which
    // passes most over without a call, and only those left are made ready
    // to match, once. A letter more
    // searches only those the letters before it found, since each word
    // matches where a word starts and so does all it starts with: walking
    // everyone each key was the launcher's dearest list, 8 ms a key and
    // 553 ms at worst (launcher-trials.json, 2026-10-25 to 27). Only who
    // matched is kept; how each stands, and so their order, is read anew.
    readonly property var lately: ({ typed: null, among: null, found: [] })

    function gather(typed) {
        seek(typed);
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const rows = known;
        const missed = { score: -1, grade: 3 };
        const found = [];
        const matched = [];
        let grade = 3;
        const among = everyone();
        const narrowing = lately.among === among && lately.typed !== null
            && String(typed).toLowerCase().startsWith(lately.typed);
        for (const row of narrowing ? lately.found : among) {
            const byName = (row.named & need) === need
                ? Fuzzy.match(row.prepared, words, false) : missed;
            const byAddress = (row.mailed & need) === need
                ? Fuzzy.match(row.also, words, false) : missed;
            const best = byName.score >= byAddress.score ? byName : byAddress;
            if (best.score < 0)
                continue;
            matched.push(row);
            grade = Math.min(grade, best.grade);
            const listed = rows[row.key];
            found.push({ score: best.score, grade: best.grade, row: listed ?? row,
                listed: listed !== undefined, at: found.length });
        }
        lately.typed = String(typed).toLowerCase();
        lately.among = among;
        lately.found = matched;
        // Those listed first, then the best match, and among equals the
        // order the sources know them in.
        const kept = found.filter(each => each.grade === grade)
            .sort((a, b) => Number(b.listed) - Number(a.listed) || b.score - a.score
                || a.at - b.at);
        return { rows: kept.map(each => each.row), grade: grade };
    }

    function everything() {
        return provider.wanted ? all : listedNow();
    }

    // Home names those listed and says how many from the people themselves,
    // making none of their rows, so People isn't wanted there (Wanted.js)
    // and its rows wait to be searched or stepped into. Read for its names,
    // every opening listed them all again, each person's text written to
    // see whether their row could be kept, and the first after each start
    // made every row, 486 on Joel's laptop (2026-11-10).
    saysAtHome: true
    // Yet those listed are made as the shell starts, in People's turn of
    // it (Launcher.qml's warmNext), and kept, so the first letter typed at
    // home finds them made and a change since remakes only those it
    // changed. Made by that letter instead, 121 rows held it 15 ms on
    // Joel's laptop (his journal, 2026-11-11 23:10; RCTRF8's d181771a).
    onWarmingChanged: {
        if (provider.warming)
            listedNow();
    }

    // Those listed, in the daemon's order, each as it told them, read
    // again only once it says a change. While a source keeps something
    // of its own the daemon hasn't heard yet, as a favourite changed here
    // is until Tuple says it, they're merged and ordered here instead, so
    // the row moves at once.
    readonly property var told: ({ revision: -1, listed: [] })
    function listedPersons() {
        if (sources.some(source => Object.keys(source.standing ?? {}).length > 0)) {
            told.revision = -1;
            return listedHere();
        }
        if (told.revision !== listedRevision) {
            told.listed = placing.keys
                .map(key => lister.held[rowsHeld[key]?.thing]?.person).filter(Boolean);
            told.revision = listedRevision;
        }
        return told.listed;
    }

    // Those listed, merged and ordered here, kept while everyone and how
    // they stand are the same and nothing with anyone has started or
    // ended since. Plain, so keeping them notifies nothing.
    readonly property var listing: ({ persons: null, stands: null, until: 0, listed: [] })
    function listedHere() {
        const now = Date.now();
        const kept = listing;
        const everyone = merged();
        if (kept.persons !== everyone.persons || kept.stands !== everyone.stands
            || now >= kept.until) {
            kept.listed = Person.ordered(everyone.persons.filter(person => Person.listed(person,
                standOf(person), now)), person => standOf(person), now);
            kept.persons = everyone.persons;
            kept.stands = everyone.stands;
            kept.until = Person.changesAt(everyone.persons, now);
        }
        return kept.listed;
    }
    // Home says the people daemon isn't answering rather than a count of
    // whom it last listed, so a down daemon is never silent.
    function homeSays() {
        if (pressing !== "")
            return pressing;
        const count = listedPersons().length;
        return count === 0 && (collection.none ?? "") !== "" ? null : countedAs(count);
    }
    function memberNames() {
        return listedPersons().map(person => person.name);
    }

    // Those listed, by their rows' keys alone, for a pick (Launcher.qml's
    // knowing): a person's row asks for no confirming, so its key is all
    // a pick reads. Read for their rows, a pick made every listed person's
    // row while People wasn't wanted, 116 at one of Joel's (2026-11-10).
    function keysKnown() {
        return provider.wanted ? all
            : listedPersons().map(person => ({ key: `person:${person.key}` }));
    }

    // A search looks through everyone, not only those listed.
    function looksThrough() {
        return everyone();
    }

    // A source that only follows what it knows while someone might look,
    // as Tuple's does, is woken as the launcher opens.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                for (const source of provider.sources)
                    source.woken();
        }
    }
}
