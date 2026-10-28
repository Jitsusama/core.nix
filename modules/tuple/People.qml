// Calls, Tuple's place in the launcher, a collection found by typing
// "call", "pair" or "tuple": first what can be done about calls now, as Tuple.qml
// says it, so a ringing call, or the call Joel's in, leads, and out of one
// his room does; then the people to pair with. Its row at home says how
// Tuple stands: who's calling, in a call, or how many can pair.
//
// The people are everyone Tuple lists as a contact, as
// tuple-people says them (people.rs). Tuple's own `contacts follow` says
// every contact again, over a megabyte of them, each time anyone's
// presence changes; the helper reads all of that so the shell doesn't,
// and says only what changed: everyone, which in practice is once, and
// how those Tuple knows stand, each time one does. It starts as the
// launcher first opens, and again as it next opens after it ended, as it
// does when Tuple's daemon stops. Any tuple command starts a daemon, so
// the helper runs Tuple only while its daemon answers, and a stopped
// Tuple stays stopped. Once the follow ends, how anyone stands is no
// longer known, so only who's a favourite or recent is kept.
// With nothing typed it holds the favourites, then those called lately,
// each in order of who can pair: available, in a call, away, then
// offline. Typing finds anyone by name or email, those Tuple knows first.
// Each says how they stand, and nothing when Tuple doesn't know. Return
// calls one who isn't in a call and joins one who is; the right arrow
// steps into calling or joining, making them a favourite or not, and
// copying their email.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "person"

    // Named Calls (Words.qml): the program's name finds it but never
    // names it.
    collection: Words.collection("people", {
        ask: Tuple.calling ? "what to do in the call?" : "who to pair with?"
    })

    // What can be done about calls now, which leads the place.
    readonly property var moment: Tuple.controls.map(control => thing(Object.assign({
        what: "call"
    }, control)))
    onMomentChanged: changed()

    // Everyone as a row each, made again only when the helper says
    // everyone again, and who they are by id.
    property var everyone: []
    property var people: ({})
    // How those Tuple knows stand, by id: { status, favourite, recent }.
    property var standing: ({})

    // How each stands, in words and in the order of who can pair first.
    // Tuple says "unknown" of anyone not a favourite, which says nothing.
    readonly property var statuses: ({
        online: { said: "available", order: 0 },
        busy: { said: "in a call", order: 1 },
        unavailable: { said: "away", order: 2 },
        offline: { said: "offline", order: 3 }
    })

    function how(stands) {
        return statuses[stands ? stands.status : ""] ?? { said: "", order: 4 };
    }

    function rank(stands) {
        return stands.favourite ? 0 : stands.recent ? 1 : 2;
    }

    // Tuple finds someone by their email; the id Tuple knows them by is
    // the one left when they have none.
    function who(person) {
        return person.email || String(person.id);
    }

    // Those Tuple knows, as they stand: favourites, then those called
    // lately, each by who can pair first and then by name. One the helper
    // hasn't named is left out.
    readonly property var all: Object.keys(standing)
        .filter(id => people[id] !== undefined)
        .sort((a, b) => rank(standing[a]) - rank(standing[b])
            || how(standing[a]).order - how(standing[b]).order
            || people[a].name.localeCompare(people[b].name))
        .map(id => made(people[id], standing[id]))
    onAllChanged: changed()

    // Their rows by key, so a search finds them as they stand.
    readonly property var known: {
        const by = {};
        for (const row of all)
            by[row.key] = row;
        return by;
    }

    function made(person, stands) {
        const busy = stands !== null && stands.status === "busy";
        const row = member({
            key: `person:${person.id}`,
            verb: busy ? "join" : "call",
            name: person.name,
            person: person,
            known: stands !== null,
            status: stands ? stands.status : "unknown",
            glyph: stands !== null && stands.favourite ? Theme.glyph.favourite : Theme.glyph.person,
            hue: "person",
            side: how(stands).said,
            note: person.email,
            later: true,
            run: () => provider.pair(person),
            verbs: () => provider.verbs(person)
        });
        // Their email, made ready to match when first searched, as the
        // name is.
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.prepare(person.email))
        });
        return row;
    }

    // How they stand now, which a row made before may not say.
    function standingOf(person) {
        return standing[person.id] ?? null;
    }

    function pair(person) {
        const busy = standingOf(person)?.status === "busy";
        Niri.spawn(["tuple", "call", busy ? "join" : "start", who(person)]);
    }

    // Marks them a favourite or not at once, as Tuple will once it's told,
    // so the row moves now rather than when Tuple next says.
    function favour(person, wanted) {
        Niri.spawn(["tuple", "contacts", wanted ? "favorite" : "unfavorite", who(person)]);
        const now = Object.assign({ status: "unknown", recent: false }, standingOf(person),
            { favourite: wanted });
        const next = Object.assign({}, standing);
        if (now.favourite || now.recent || now.status !== "unknown")
            next[person.id] = now;
        else
            delete next[person.id];
        standing = next;
    }

    function verbs(person) {
        const stood = standingOf(person);
        const busy = stood !== null && stood.status === "busy";
        const favourite = stood !== null && stood.favourite;
        return [thing({
            key: busy ? "verb:person:join" : "verb:person:call",
            name: busy ? "Join Their Call" : "Call Them",
            side: how(stood).said,
            glyph: busy ? Theme.glyph.joinCall : Theme.glyph.call,
            hue: "action",
            later: true,
            run: () => provider.pair(person)
        }), thing({
            key: "verb:person:favourite",
            name: favourite ? "Stop Them Being a Favourite" : "Make Them a Favourite",
            glyph: favourite ? Theme.glyph.notFavourite : Theme.glyph.favourite,
            hue: "action",
            run: () => provider.favour(person, !favourite)
        })].concat(person.email ? [copyRow({
            key: "fact:person:email",
            name: person.email,
            side: "their email",
            glyph: Theme.glyph.email
        })] : []);
    }

    // Inside the place, what's typed finds what can be done about calls
    // as well as people, those first, at whichever is the finer match.
    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of moment) {
            const byName = Fuzzy.match(row.prepared, words, false);
            const best = byName.score >= 0 || !row.keywords ? byName
                : Fuzzy.match(row.keywords, words, false);
            if (best.score >= 0)
                found.push({ row: row, grade: best.grade });
        }
        const people = gather(typed);
        const grade = Math.min(people.grade, ...found.map(f => f.grade));
        return {
            rows: found.filter(f => f.grade === grade).map(f => f.row)
                .concat(people.grade === grade ? people.rows : []),
            grade: grade
        };
    }

    // In the main search only people, since what can be done about calls
    // is found there on its own (Calls.qml).
    // Everyone is searched, thousands of them, so each name and email's
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
            const person = row.person;
            const byName = (person.named & need) === need
                ? Fuzzy.match(row.prepared, words, false) : missed;
            const byEmail = (person.mailed & need) === need
                ? Fuzzy.match(row.also, words, false) : missed;
            const best = byName.score >= byEmail.score ? byName : byEmail;
            if (best.score < 0)
                continue;
            matched.push(row);
            grade = Math.min(grade, best.grade);
            found.push({ score: best.score, grade: best.grade, row: rows[row.key] ?? row, at: found.length });
        }
        lately.typed = String(typed).toLowerCase();
        lately.among = everyone;
        lately.found = matched;
        // Those Tuple knows first, then the best match, and among equals
        // the order Tuple lists them in.
        const kept = found.filter(f => f.grade === grade)
            .sort((a, b) => Number(b.row.known) - Number(a.row.known) || b.score - a.score || a.at - b.at);
        return { rows: kept.map(f => f.row), grade: grade };
    }

    function everything() {
        return moment.concat(all);
    }

    function counted() {
        if (Tuple.standing !== "")
            return Tuple.standing;
        if (stopped)
            return "not running";
        if (everyone.length === 0)
            return "empty";
        return `${all.filter(row => row.status === "online").length} available`;
    }

    // Each line says either everyone or how those Tuple knows stand.
    function heard(line) {
        let said;
        try {
            said = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (Array.isArray(said?.people)) {
            const byId = {};
            const rows = [];
            for (const [id, name, email] of said.people) {
                const person = {
                    id: id,
                    name: name,
                    email: email,
                    named: Fuzzy.letters([name.toLowerCase()]),
                    mailed: Fuzzy.letters([email.toLowerCase()])
                };
                byId[id] = person;
                rows.push(made(person, null));
            }
            provider.people = byId;
            provider.everyone = rows;
        } else if (Array.isArray(said?.standing)) {
            const stood = {};
            for (const [id, status, favourite, recent] of said.standing)
                stood[id] = { status: status, favourite: favourite, recent: recent };
            provider.standing = stood;
        }
    }

    // Whether the helper last found Tuple's daemon stopped.
    property bool stopped: false

    // What the helper says when no daemon answers (people.rs).
    readonly property int notRunning: 69

    function ended(code) {
        const kept = {};
        for (const id in standing) {
            const stands = standing[id];
            if (stands.favourite || stands.recent)
                kept[id] = { status: "unknown", favourite: stands.favourite, recent: stands.recent };
        }
        standing = kept;
        stopped = code === notRunning;
    }

    // The helper's process, for a test to stop as a restart of the shell
    // would.
    readonly property Process following: stream

    Process {
        id: stream
        // Tuple's daemon listens here, as tuple-0 for its first account.
        command: ["tuple-people", "tuple", `${Quickshell.env("XDG_RUNTIME_DIR")}/tuple-0`]
        stdout: SplitParser {
            onRead: line => provider.heard(line)
        }
        onStarted: provider.stopped = false
        onExited: code => provider.ended(code)
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open && !stream.running)
                stream.running = true;
        }
    }
}
