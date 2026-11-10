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
// chat), their page in each source that keeps one, their next meeting,
// their Recent, the things lately with them, whether they're a favourite,
// then each address to copy, last as every Copy is. Each thing is drawn
// by its own row, so a mail is Mail's row and a meeting the meeting's.
// Their details, every address and each source's facts, are in the
// drawer beside the row.
//
// A source the people daemon holds (`daemonHolds`, as Tuple's) is read from
// the daemon, never held whole here: only those it lists with nothing
// typed, watched as they change, and those a search finds, asked of the
// daemon as each text is typed. Holding Tuple's 7,731 contacts here made
// 23,276 rows at each opening and held Joel's first key a second
// (2026-10-31, 09:31).
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Person.js" as Person
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
    // (its listing kind): one person each, how they stand, and their
    // order, told whole once and after only who changed, each with what
    // every source told of them. So the shell neither merges nor orders
    // them: with 600 people in Qt's engine, merging everyone took the GUI
    // thread 26 to 38 ms, and ordering those listed 19 more (2026-11-10). It's said when the daemon stops answering, and those
    // it last listed stay listed until it answers again.
    Daemon {
        id: lister

        socket: book.socket
        kind: "listing"
        answersWhole: true
        onStateChanged: {
            if (state === "not answering")
                console.warn("People: the people daemon isn't answering, "
                    + "so those listed are as it last said");
        }
    }
    // The record the listing says its order in.
    readonly property string orderKey: "people:order"

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
    // As many as a search is answered with, as the daemon does unasked.
    readonly property int soughtMost: 50

    // Everything the people are made from: the daemon's records, and each
    // source's people and how they stand. A change to any marks what's
    // made of them stale, to be made again only once something asks, so
    // a change nobody looks at merges no one: merging everyone took 26 to
    // 38 ms of the GUI thread at every change to any source's people,
    // open or shut, 600 of them in Qt's engine (2026-11-10).
    readonly property var heard: [lister.revision, sought].concat(sources.map(source =>
        [source.people, source.stands, source.standing]))
    onHeardChanged: {
        merging.stale = true;
        rowsOfEveryone.stale = true;
        heardRevision += 1;
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
                people[name].push(email !== ""
                    ? { emails: [email], name: entry.name ?? "", recent: entry.recent === true }
                    : { emails: [], id: id, name: entry.name ?? "", recent: entry.recent === true });
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
    function seek(typed) {
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
            sought = { words: words, records: answer.records ?? [] };
            foundMore(Date.now() - seekAt < freshFor);
        });
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
            made.rows = persons().map(person => keptOr(made.keeping, person.key,
                JSON.stringify(person), () => provider.made(person, Person.standing(person, []))));
            keptOnly(made.keeping);
            made.stale = false;
        }
        return made.rows;
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
        if (listedRows.from === listed)
            return listedRows.rows;
        keepingBegun(keptListed);
        const rows = listed.map(person => {
            const stood = person.stood ?? standOf(person);
            return keptOr(keptListed, person.key, JSON.stringify([person, stood]),
                () => made(person.stood ? withFacts(person) : person, stood));
        });
        keptOnly(keptListed);
        listedRows.from = listed;
        listedRows.rows = rows;
        return rows;
    }
    readonly property var keptListed: keeper()
    readonly property var listedRows: ({ from: null, rows: [] })

    // Someone the daemon listed, with the facts each source adds read
    // from everyone merged as their drawer is shown, since the daemon
    // keeps no source's facts: so only a drawer shown merges anyone.
    function withFacts(person) {
        const shown = Object.assign({}, person);
        Object.defineProperty(shown, "facts", {
            get: () => whoIs(person.email)?.facts ?? []
        });
        return shown;
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
    function made(person, stood) {
        const row = member({
            key: `person:${person.key}`,
            component: "person",
            name: person.name,
            person: person,
            glyph: stood.favourite ? Theme.glyph.favourite : Theme.glyph.person,
            hue: "person",
            side: stood.said,
            note: person.name === person.email ? "" : person.email,
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
        // is made again once its minute has passed.
        let details = null;
        let detailsAt = -1;
        Object.defineProperty(row, "drawer", {
            enumerable: true,
            get: () => {
                const minute = Math.floor(Date.now() / 60000);
                if (details === null || detailsAt !== minute) {
                    details = Person.details(person, zone => provider.there(zone));
                    detailsAt = minute;
                }
                return details;
            }
        });
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

    // What the right arrow lists: every way to reach them, their next
    // meeting, what of theirs is unread, what Joel had with them lately
    // (Recent), whether they're a favourite,
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
        return acts(person).concat(pages(person), next(person), unread(person),
            recent(person), favourite(person), addresses);
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

    // Who an address is, as People knows them, { key, name, emails, ... },
    // with no row made: for a count of a meeting's guests or a call's name
    // (Calendar.qml, Transcripts.qml), which made each one's whole row; null
    // when no source knows them.
    function whoIs(email) {
        return merged().byEmail[String(email ?? "").trim().toLowerCase()] ?? null;
    }

    // Their next meeting, drawn as a meeting is everywhere, the soonest any
    // source has; a meeting to come isn't Recent.
    function next(person) {
        const coming = sources.map(source => source.next(person, provider))
            .filter(Boolean).sort((a, b) => a.at - b.at)[0];
        return coming ? [coming.row] : [];
    }

    // Their Recent: what Joel had with them across every source, laid out
    // as every Recent is (Recent.js): the last nine, newest first, each
    // thing once by its row's key, each its own thing's row. It stands
    // whether or not there's any, saying how many, so it's always where
    // it was. It's gathered once as they're stepped into, and stepping
    // into it lists what was gathered then.
    function recent(person) {
        const rows = Recent.listed(sources.map((source, which) => source
            .recent(person, provider)
            .map((one, at) => ({ key: one.row?.key || `${which}:${at}`, at: one.at,
                row: one.row }))), []).map(one => one.row);
        const stands = Recent.said(rows.map(row => row.name), false,
            Words.of("person:recent").said);
        return [holding("person:recent", rows, stands.side, stands.held)];
    }

    // Their Unread: what of theirs waits on Joel, unread mail, newest
    // first, standing only while there's any, how many at its side and
    // their names in its drawer, as Recent's and Slack's Unread say
    // theirs. It comes before Recent, which holds only
    // what he's had with them.
    function unread(person) {
        const rows = [].concat(...sources.map(source => source.unread(person, provider)))
            .sort((a, b) => b.at - a.at).map(one => one.row);
        const stands = Recent.said(rows.map(row => row.name), false, {});
        return rows.length === 0 ? []
            : [holding("person:unread", rows, stands.side, stands.held)];
    }

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
    function search(typed) {
        return gather(typed);
    }

    // Everyone shown is searched, those the sources give and the daemon's
    // answers, so each name and address's letters are tested first, which passes most over without a call,
    // and only those left are made ready to match, once. A letter more
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
        if (told.revision !== lister.revision) {
            told.listed = (lister.held[orderKey]?.order ?? [])
                .map(key => lister.held[key]?.person).filter(Boolean);
            told.revision = lister.revision;
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
    function homeSays() {
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
