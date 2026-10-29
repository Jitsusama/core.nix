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
import qs
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

    // Everyone, one person each, joined again only when a source says who
    // it knows again, which is seldom; and each as a row, by key, its name
    // and addresses made ready to match only when first searched.
    readonly property var persons: Person.merged(sources.map(source => ({
        name: source.settings.name,
        people: source.people
    })))
    readonly property var everyone: persons.map(person => made(person, Person.standing(person, [])))

    // How each stands, which changes often, read from every source in
    // order, so the first that says is believed.
    readonly property var stands: sources.map(source => source.stands)

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
            drawer: Person.details(person, zone => provider.there(zone)),
            nested: true,
            later: true,
            verb: "more",
            stays: true,
            run: () => provider.launcher?.enter(row),
            verbs: () => provider.inside(person),
            targets: () => provider.inside(person)
        });
        // Their addresses, made ready to match when first searched, as the
        // name is.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.prepare(person.emails.join(" ")))
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
    // launcher.
    function pages(person) {
        return sources.map(source => ({ source: source, link: source.page(person) }))
            .filter(each => each.link !== "" && each.source.called !== "")
            .map(each => thing({
                key: `verb:person:page:${each.source.settings.name}`,
                name: Words.name("verb:person:page", "", { source: each.source.called }),
                verb: Words.verb("verb:person:page"),
                glyph: Words.glyph("verb:person:page"),
                hue: "action",
                later: true,
                run: () => Niri.spawn(["xdg-open", each.link])
            }));
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
    // when no source has any.
    function recent(person) {
        const had = () => sources.reduce((each, source) => each.concat(source.recent(person,
            provider)), []).sort((a, b) => b.at - a.at).map(one => one.row);
        const count = had().length;
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

    // Everyone is searched, thousands of them, so each name and address's
    // letters are tested first, which passes most over without a call,
    // and only those left are made ready to match, once. A letter more
    // searches only those the letters before it found, since each word
    // matches where a word starts and so does all it starts with: walking
    // everyone each key was the launcher's dearest list, 8 ms a key and
    // 553 ms at worst (launcher-trials.json, 2026-10-25 to 27). Only who
    // matched is kept; how each stands, and so their order, is read anew.
    readonly property var lately: ({ typed: null, among: null, found: [] })

    function gather(typed) {
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
