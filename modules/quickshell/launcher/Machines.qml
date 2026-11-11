// Machines, a collection: every machine of the realms this one links, as
// each realm's mycelium node serves them (kind `machine`, mycelium's
// PROTOCOL.md), told every change as it comes rather than read again as
// the launcher opens. This machine comes first and its Return opens the
// Machine place, one place per thing; each other one says what it is and
// how it stands, and its Return steps into its figures as a page, each
// copying its value, then its acts: Start a Session Here, Update It, which
// asks once on the command line, and Copy the Name. Every row, this
// machine's too, has the same drawer of its figures. One out of reach dims,
// says since when and offers no act (VKCF3Z's rulings of 2026-11-07, in
// KP0L6A's design-machines.md, and 2026-11-09).
// RealmMachines.js says each row and drawer, and how an update ended.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../When.js" as When
import "RealmMachines.js" as Listed

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
            id: node
            required property string modelData
            readonly property string realm: modelData
            socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/mycelium/${modelData}.sock`
            kind: "machine"
            changesDiffer: true
            answersWhole: true
            onRevisionChanged: provider.hear(realm, held)
            Component.onDestruction: provider.hear(realm, null)

            // An update of one of its machines, asked on this node's own
            // connection, answered once its updater says how it ended.
            Connections {
                target: provider

                function onUpdateAsked(realm, machine) {
                    if (realm !== node.realm)
                        return;
                    node.find({ ask: "update", machine: machine }, answer => {
                        provider.updating = "";
                        Report.say(Listed.updateEnded(machine, answer,
                            Words.of("machines").said ?? {}));
                    });
                }
            }
        }
    }

    // What each realm's node holds, by realm, told by the node itself as
    // it changes, since an Instantiator's objects are only QObjects to the
    // linter, which can't see a Daemon's records on one.
    property var heldBy: ({})
    function hear(realm, held) {
        const next = Object.assign({}, heldBy);
        if (held)
            next[realm] = held;
        else
            delete next[realm];
        heldBy = next;
    }

    // Every record the nodes hold, each with the realm that said it. A
    // node too old to list this machine leaves it out, so it's said from
    // what the shell knows of itself, its name, until one does.
    readonly property var records: {
        const all = [];
        for (const realm of Object.keys(heldBy)) {
            for (const record of Object.values(heldBy[realm]))
                all.push(Object.assign({}, record, { realm }));
        }
        if (Sessions.machine !== "" && !all.some(record => record.here === true))
            all.push({ name: Sessions.machine, here: true, state: "ready",
                realm: Sessions.realm, measured_at: new Date().toISOString() });
        return all;
    }

    // What RealmMachines.js needs to say them: the words, the realms, now, this
    // machine's core, how the disk guard bands a disk, and a moment's day and
    // time as the launcher says them everywhere (When.js).
    function placeNow(listed) {
        const here = listed.find(record => record.here === true);
        const said = Words.of("machines").said ?? {};
        return {
            said: said,
            day: (at, now) => When.day(at, now, {
                yesterday: said.yesterday ?? "",
                date: (date, format) => Qt.formatDate(date, format)
            }),
            clock: at => Qt.formatDateTime(new Date(at), "HH:mm"),
            named: id => Words.name(`machines:${id}`),
            now: Date.now(),
            realms: realms.length,
            core: here?.updated?.followed?.core ?? "",
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

    // This machine: Return opens the Machine place, one place per thing,
    // and its drawer is every machine's, one component per thing. Its side
    // leads with what tells it apart, what it is giving way first.
    function hereRow(record, place) {
        const items = Listed.hereItems(record, place);
        const row = member({
            thing: true,
            key: `machines:${record.name}`,
            name: record.name,
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            drawer: Listed.drawer(record, place),
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

    // Its figures as a page, each copying its value, then its acts, the
    // last Copy the Name.
    function inside(record, drawn) {
        const acts = Listed.reached(record) ? [startHere(record), updateIt(record)] : [];
        return drawn.facts.map(fact => copyRow(Object.assign({
            key: `machines:${record.name}:${fact.label}`,
            name: fact.label,
            side: fact.value,
            copy: fact.value
        }, fact.mark ? { glyph: Words.glyph(fact.mark) } : {}))).concat(acts, [copyRow({
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

    // Starts a session there, in the same folder under its home as this
    // machine's home, held by its node, its window opening here.
    function startHere(record) {
        const id = "verb:machines:start";
        return thing({
            key: `${id}:${record.name}`,
            name: Words.name(id),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            later: true,
            run: () => Sessions.startOn(record.name, "", Quickshell.env("HOME"), record.realm)
        });
    }

    // The machine whose update is going now, or "": one at a time, since
    // each takes minutes and says how it ended on its own.
    property string updating: ""

    // Updates it to its configuration's main, which can't be taken back,
    // so it asks once on the command line as every such act does, the act
    // on its thing then y/n: "Update skiff? (y/n)" (docs/design.md's Undo;
    // VKCF3Z, 2026-11-09). While it goes, its foot says so.
    function updateIt(record) {
        const id = "verb:machines:update";
        const said = Words.of("machines").said ?? {};
        const going = () => (provider.updating === record.name ? said.updating : "");
        return thing({
            key: `${id}:${record.name}`,
            name: Words.name(id),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            stays: true,
            state: going,
            gone: updating !== "",
            asked: String(Words.of(id).asked ?? "").replace("{name}", record.name),
            run: () => provider.update(record)
        });
    }

    // Asks its realm's node to update it over the node's held socket,
    // rather than starting a program to ask it, as the launcher starts
    // none to get or do anything. A realm no node answers for says so.
    signal updateAsked(string realm, string machine)
    function update(record) {
        if (updating !== "")
            return;
        if (!realms.includes(record.realm)) {
            Report.say(Listed.updateEnded(record.name, null, Words.of("machines").said ?? {}));
            return;
        }
        updating = record.name;
        updateAsked(record.realm, record.name);
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
