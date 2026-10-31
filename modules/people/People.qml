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

    // The people daemon: those a held source lists, watched as they
    // change, and a search's answers, given to the search alone.
    Daemon {
        id: book

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/people.sock`
        kind: "person"
        watching: ({ listed: true })
    }

    // Counts each change the daemon says, for a source that keeps
    // something of its own until then (Tuple's favourites changed here).
    readonly property int revision: book.revision

    // The names of the sources the daemon holds.
    readonly property var heldBy: sources.filter(source => source.daemonHolds)
        .map(source => source.settings.name)

    // What the daemon's search last found, for the words asked, best
    // first; and the words asked last, so each text is asked once.
    property var sought: ({ words: "", records: [] })
    property string seeking: ""
    // As many as a search is answered with, as the daemon does unasked.
    readonly property int soughtMost: 50

    // Every record read from the daemon, once each: the latest search's
    // first, in its order, then those listed.
    readonly property var records: {
        const by = {};
        const all = [];
        const held = book.revision >= 0 ? book.held : {};
        for (const record of sought.records.concat(Object.values(held))) {
            if (record.gone || record.key in by)
                continue;
            by[record.key] = record;
            all.push(record);
        }
        return all;
    }
    readonly property var recordOf: {
        const by = {};
        for (const record of records)
            by[record.key] = record;
        return by;
    }

    // A held source's people and stands, from the records read, as a
    // source of its own gives them.
    readonly property var fromDaemon: {
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
            const entry = recordOf[key]?.sources?.[name];
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
        book.find({ ask: "search", kind: "person", words: words, limit: soughtMost }, answer => {
            if (seeking !== words)
                return;
            if (answer === null) {
                seeking = "";
                return;
            }
            sought = { words: words, records: answer.records ?? [] };
            foundMore();
        });
    }

    // Everyone shown, one person each: those the sources give, and those
    // read from the daemon; and each as a row, by key, its name and
    // addresses made ready to match only when first searched.
    readonly property var persons: Person.merged(sources.map(source => ({
        name: source.settings.name,
        people: source.daemonHolds ? fromDaemon.people[source.settings.name] : source.people
    })))
    readonly property var everyone: persons.map(person => made(person, Person.standing(person, [])))

    // How each stands, which changes often, read from every source in
    // order, so the first that says is believed.
    readonly property var stands: sources.map(source => (source.daemonHolds
        ? fromDaemon.stands[source.settings.name] : source.stands))

    function standOf(person) {
        return Person.standing(person, stands);
    }

    // Those listed with nothing typed, as they stand now, the last thing
    // with each measured as they're listed rather than kept up with the
    // clock.
    readonly property var all: {
        const now = Date.now();
        return Person.ordered(persons.filter(person => Person.listed(person, standOf(person), now)),
            person => standOf(person), now).map(person => made(person, standOf(person)));
    }
    onAllChanged: changed()

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
        // everyone again, and so this row, which reads it then.
        let details = null;
        Object.defineProperty(row, "drawer", {
            enumerable: true,
            get: () => details ?? (details = Person.details(person, zone => provider.there(zone)))
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
    // meeting, the things lately with them, whether they're a favourite,
    // then each address to copy, last as every Copy is.
    function inside(person) {
        const addresses = person.emails.map((email, at) => copyRow({
            key: `fact:person:email:${at}`,
            name: email,
            side: person.emails.length === 1 ? "their email" : "one of their emails",
            glyph: Theme.glyph.email
        }));
        return acts(person).concat(pages(person), next(person), recent(person),
            favourite(person), addresses);
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

    // Everyone by each of their addresses, lowercased, joined again only
    // when who's known is.
    readonly property var byEmail: {
        const by = {};
        for (const person of persons) {
            for (const email of person.emails)
                by[email.toLowerCase()] = person;
        }
        return by;
    }

    // The person an address is, as their one row, for a thing that names
    // them, as a mail's sender or a meeting's guests (Mail.qml,
    // Calendar.qml): null when no source knows them, so nothing is listed
    // that can't step in. Asked of any provider holding `personOf`, so a
    // machine without People lists no one.
    function personOf(email) {
        const person = byEmail[String(email ?? "").trim().toLowerCase()];
        return person ? made(person, standOf(person)) : null;
    }

    // Their next meeting, drawn as a meeting is everywhere, the soonest any
    // source has; a meeting to come isn't Recent.
    function next(person) {
        const coming = sources.map(source => source.next(person, provider))
            .filter(Boolean).sort((a, b) => a.at - b.at)[0];
        return coming ? [coming.row] : [];
    }

    // Their Recent: what each source has had with them, newest first, each
    // its own thing's row, the same word as a program's Recent, or nothing
    // when no source has any. It's gathered once as they're stepped into,
    // and stepping into it lists what was gathered then.
    function recent(person) {
        const gathered = [];
        for (const source of sources) {
            for (const one of source.recent(person, provider))
                gathered.push(one);
        }
        const rows = gathered.sort((a, b) => b.at - a.at).map(one => one.row);
        const had = () => rows;
        const count = rows.length;
        if (count === 0)
            return [];
        // Its rows are both what it holds and what it steps into, as a
        // meeting's See It's are, so it steps in at any level: inside
        // People, where a row's insides are its verbs, and inside a
        // person found from the top, where they're its targets.
        const row = thing({
            key: "person:recent",
            name: Words.name("person:recent"),
            verb: Words.verb("person:recent"),
            glyph: Words.glyph("person:recent"),
            hue: "person",
            count: String(count),
            nested: true,
            stays: true,
            verbs: had,
            targets: had,
            run: () => provider.launcher?.enter(row)
        });
        return [row];
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
        const narrowing = lately.among === everyone && lately.typed !== null
            && String(typed).toLowerCase().startsWith(lately.typed);
        for (const row of narrowing ? lately.found : everyone) {
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
        lately.among = everyone;
        lately.found = matched;
        // Those listed first, then the best match, and among equals the
        // order the sources know them in.
        const kept = found.filter(each => each.grade === grade)
            .sort((a, b) => Number(b.listed) - Number(a.listed) || b.score - a.score
                || a.at - b.at);
        return { rows: kept.map(each => each.row), grade: grade };
    }

    function everything() {
        return all;
    }

    // A search looks through everyone, not only those listed.
    function looksThrough() {
        return everyone;
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
