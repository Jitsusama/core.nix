// Machines, a collection: every machine of the realms this one links, as
// each realm's mycelium node serves them (kind `machine`, mycelium's
// PROTOCOL.md), told every change as it comes rather than read again as
// the launcher opens. This machine comes first and its Return opens the
// Machine place, one place per thing; each other one says what it is and
// how it stands, its drawer its figures, and its Return steps into those
// figures as a page, each copying its value, then Copy the Name. One out
// of reach dims and says since when (VKCF3Z's ruling of 2026-11-07, in
// KP0L6A's design-machines.md). Machines.js says each row and drawer.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Ago.js" as Ago
import "Machines.js" as Listed

Provider {
    id: provider

    what: Words.of("machines").what

    collection: Words.collection("machines")

    // The realms this machine's nodes run, as its mycelium configuration
    // names them (home.nix), each node's socket by its realm's name.
    readonly property var realms: settings.realms ?? []

    // Each realm's node, watched whole and then told each change as only
    // what differs. What was held stays held while a node isn't
    // answering, as while a switch restarts it.
    Instantiator {
        id: nodes
        model: Quickshell.env("XDG_RUNTIME_DIR") ? provider.realms : []

        Daemon {
            required property string modelData
            readonly property string realm: modelData
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${modelData}.sock`
            kind: "machine"
            changesDiffer: true
            answersWhole: true
            onRevisionChanged: provider.heard += 1
        }
    }
    property int heard: 0

    // Every record the nodes hold, each with the realm that said it. A
    // node too old to list this machine leaves it out, so it's said from
    // what the shell knows of itself, its name, until one does.
    readonly property var records: {
        void heard;
        void nodes.count;
        const all = [];
        for (let index = 0; index < nodes.count; index += 1) {
            const node = nodes.objectAt(index);
            for (const record of Object.values(node?.held ?? {}))
                all.push(Object.assign({}, record, { realm: node.realm }));
        }
        if (Sessions.machine !== "" && !all.some(record => record.here === true))
            all.push({ name: Sessions.machine, here: true, state: "ready",
                realm: Sessions.realm, measured_at: new Date().toISOString() });
        return all;
    }

    // What Machines.js needs to say them: the words, the realms, now, this
    // machine's core and how the disk guard bands a disk.
    function placeNow(listed) {
        const here = listed.find(record => record.here === true);
        return {
            said: Words.of("machines").said ?? {},
            named: id => Words.name(`machines:${id}`),
            now: Date.now(),
            realms: realms.length,
            core: here?.updated?.followed?.core ?? "",
            ago: Ago.ago,
            band: band => (band === "" || band === "healthy" ? ""
                : (Words.of("machine:disk").said ?? {})[band] ?? band)
        };
    }

    readonly property var all: {
        const listed = Listed.ordered(records);
        const place = placeNow(listed);
        return listed.map(record => (record.here === true ? hereRow(record, place)
            : made(record, place)));
    }
    onAllChanged: changedTo(all)

    // This machine: Return opens the Machine place, one place per thing.
    function hereRow(record, place) {
        const row = member({
            thing: true,
            key: `machines:${record.name}`,
            name: record.name,
            note: Listed.note(record, place),
            side: Listed.side(record, place),
            glyph: Words.glyph("machines"),
            hue: "machine",
            verb: "go to",
            trail: ["category:computer"],
            spot: "",
            stays: true,
            changes: false,
            run: () => provider.launcher?.goTo(row)
        });
        return row;
    }

    // Another machine: its drawer, and Return stepping into its figures,
    // then Copy the Name. Out of reach, it dims and offers no act.
    function made(record, place) {
        const drawn = Listed.drawer(record, place);
        const reached = Listed.reached(record);
        const row = thingRow({
            key: `machines:${record.name}`,
            name: record.name,
            note: Listed.note(record, place),
            side: Listed.side(record, place),
            glyph: Words.glyph("machines"),
            gone: !reached,
            drawer: drawn
        }, () => provider.inside(record, drawn));
        let also = null;
        Object.defineProperty(row, "also", {
            enumerable: true,
            get: () => also ?? (also = Fuzzy.kept(`${row.note} ${row.side}`))
        });
        return row;
    }

    // Its figures as a page, each copying its value, then Copy the Name.
    function inside(record, drawn) {
        return drawn.facts.map(fact => copyRow({
            key: `machines:${record.name}:${fact.label}`,
            name: fact.label,
            side: fact.value,
            copy: fact.value
        })).concat([copyRow({
            key: `verb:machines:copy-name:${record.name}`,
            name: Words.name("verb:machines:copy-name"),
            verb: Words.verb("verb:machines:copy-name"),
            glyph: Words.glyph("verb:machines:copy-name"),
            hue: "action",
            figure: false,
            plain: false,
            copy: record.name
        })]);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            const byName = (row.prepared.mask & need) === need
                ? Fuzzy.match(row.prepared, words, false) : { score: -1 };
            const also = row.also ?? row.prepared;
            const byWord = (also.mask & need) === need
                ? Fuzzy.match(also, words, false) : { score: -1 };
            const best = byName.score >= byWord.score ? byName : byWord;
            if (best.score >= 0)
                found.push({ score: best.score, grade: best.grade, row: row });
        }
        const grade = Math.min(3, ...found.map(f => f.grade));
        const kept = found.filter(f => f.grade === grade).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: grade };
    }

    function everything() {
        return all.slice();
    }

    // "2 reached, 1 out of reach", this machine not among them.
    function counted() {
        const others = all.filter(row => !row.key.endsWith(`:${Sessions.machine}`));
        if (others.length === 0)
            return "empty";
        const said = Words.of("machines").said ?? {};
        const gone = others.filter(row => row.gone === true).length;
        const fill = (key, count) => String(said[key] ?? "").replace("{count}", count);
        return [fill("count", others.length - gone), gone > 0 ? fill("gone", gone) : ""]
            .filter(part => part !== "").join(", ");
    }
}
