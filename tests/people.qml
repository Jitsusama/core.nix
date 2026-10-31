// Drives the launcher's People (modules/people/People.qml) with Tuple as
// its source (modules/tuple/TupleSource.qml) through Quickshell itself,
// over the real people daemon running the real tuple-people (people.rs),
// which reads a stand-in for Tuple's `contacts follow`, with niri and the
// launcher stood in. With nothing typed People lists Tuple's favourites
// and then those called lately, saying how each stands, and holds no one
// else; typing finds anyone by name or email through the daemon, those
// listed first, and a stranger saying nothing of how they stand, while
// People holds only those listed and those found; a contact with no
// email is still one person, called by Tuple's id; stepping in, calling
// one who isn't in a call and joining one who is comes first, the
// Favourite switch after, and their address last; flipping Favourite
// tells Tuple and moves their row at once; and a change of presence
// changes how they stand, as Tuple says it, forgetting the favourites
// changed here that Tuple never heard of. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: launcher

        property bool open: true
        property var entered: null

        function enter(row) {
            entered = row;
        }
    }

    Launcher.People {
        id: people

        launcher: launcher
        settings: ({
            name: "people",
            sources: [{ name: "tuple", order: 10 }, { name: "pages", order: 20 }],
            zones: Quickshell.shellPath("zones.jsonl")
        })
    }

    // Tuple's next presence, which the stand-in waits for.
    Process {
        id: touching
        command: ["touch", `${root.runtime}/next`]
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function sides(rows) {
        return rows.map(row => row.side).join(", ");
    }

    function home(name) {
        return people.all.find(row => row.name === name);
    }

    // What Return does first inside someone: call or join them.
    function reach(row) {
        row.verbs()[0].run();
        return Niri.spawned.join(" ");
    }

    // A search, once the daemon has answered it.
    function found(words) {
        return people.search(words).rows;
    }

    function answered(words) {
        return people.sought.words === words;
    }

    // Asks a search and waits for the daemon's answer, then checks it.
    function seeking(words, what, check) {
        return { what: what, run: () => people.search(words), holds: () => answered(words),
            check: check };
    }

    readonly property var steps: [
        {
            what: "with nothing typed, Tuple's favourites, then those called lately",
            holds: () => names(people.all) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore",
            check: () => {
                const open = people.all;
                expect("how each stands",
                    sides(open) === "available, in a call, away, offline", sides(open));
                expect("a favourite wears a star",
                    open[0].glyph === String.fromCodePoint(0xf04ce), open[0].glyph);
                expect("one called lately doesn't",
                    open[3].glyph === String.fromCodePoint(0xf0004), open[3].glyph);
                expect("no one else is held", people.persons.length === 4,
                    names(people.persons));
            }
        },
        seeking("eli@example.com", "a stranger found by their email", () => {
            const eli = found("eli@example.com");
            expect("found", names(eli) === "Eli Fenwick", names(eli));
            expect("who stands unknown says nothing", eli[0].side === "", eli[0].side);
        }),
        seeking("gus", "one with only an email, named by it", () => {
            const gus = found("gus");
            expect("named by it", names(gus) === "gus@example.com" && gus[0].side === "",
                `${names(gus)} ${gus[0]?.side}`);
        }),
        seeking("quill", "a name with a quote and more than ASCII", () => {
            const zoe = found("quill");
            expect("found", names(zoe) === "Zo\u00eb \"Zed\" Quill", names(zoe));
        }),
        seeking("ada", "one listed, found as they stand", () => {
            const ada = found("ada");
            expect("as they stand", names(ada) === "Ada Brook" && ada[0].side === "available",
                `${names(ada)} ${ada[0]?.side}`);
        }),
        // Among equals, the daemon's order: those found by a name before
        // those found by an address alone, gus being named by theirs, then
        // as Tuple lists them.
        seeking("example", "those listed first, then the rest, as the daemon finds them", () => {
            const all = found("example");
            expect("in order", names(all) === "Ada Brook, Ben Carrow, Cal Dunmore, Dee Ellery, "
                + "gus@example.com, Eli Fenwick, Zo\u00eb \"Zed\" Quill", names(all));
        }),
        seeking("ben", "the shell holds only those listed and those found", () => {
            expect("only those", people.persons.length === 4, names(people.persons));
            expect("found", names(found("ben")) === "Ben Carrow", names(found("ben")));
        }),
        seeking("ad", "a letter more searches only those the letters before found", () => {
            // What they found is hidden here, so a search that walked
            // everyone again would still find Ada.
            people.gather("ad");
            expect("what the letters found is kept", names(people.lately.found) === "Ada Brook",
                names(people.lately.found));
            people.lately.found = [];
            expect("a letter more searches only those", people.gather("ada").rows.length === 0,
                names(people.gather("ada").rows));
        }),
        {
            what: "stepping into those listed",
            check: () => {
                const open = people.all;
                open[0].run();
                expect("Return steps in, never calling", launcher.entered === open[0]
                    && Niri.spawned.length === 0, Niri.spawned.join(" "));
                expect("calling one who's available",
                    reach(open[0]) === "tuple call start ada@example.com", Niri.spawned.join(" "));
                expect("one in a call is joined", open[1].verbs()[0].name === "Join Their Call",
                    open[1].verbs()[0].name);
                expect("joining one in a call", reach(open[1]) === "tuple call join ben@example.com",
                    Niri.spawned.join(" "));
                expect("calling one who's offline",
                    reach(open[3]) === "tuple call start cal@example.com", Niri.spawned.join(" "));

                const steps = open[0].verbs();
                expect("their page after the ways to reach them, the address last",
                    names(steps) === "Call Them, Open in Pages, Favourite, ada@example.com",
                    names(steps));
                steps[1].run();
                expect("their page opens", steps[1].verb === "open" && Niri.spawned.join(" ")
                    === "xdg-open https://pages.example.com/ada", Niri.spawned.join(" "));
                expect("one with no page has no row for it",
                    !names(open[1].verbs()).includes("Open in"), names(open[1].verbs()));
                const calls = open[3].verbs();
                const waiting = calls[1];
                expect("a page still being found stands in its place, dimmed, saying so",
                    waiting?.name === "Open in Pages" && waiting.gone === true
                        && waiting.side === "looking in Pages",
                    `${names(calls)}; gone ${waiting?.gone}, side ${waiting?.side}`);
                Niri.spawned = [];
                waiting?.run();
                expect("and opens nothing while dimmed", Niri.spawned.length === 0,
                    Niri.spawned.join(" "));
                const facts = open[0].drawer.facts.map(fact => `${fact.label}: ${fact.value}`);
                expect("a source's facts are theirs, the time there after their zone",
                    facts[1] === "Team: Shell" && facts[2] === "Time Zone: Asia/Kolkata"
                        && /^Time There: \d\d:\d\d, 5 h 30 min ahead$/.test(facts[3]),
                    facts.join("; "));
                expect("a favourite says so", steps[2].side === "yes" && steps[2].verb === "unstar",
                    `${steps[2].side} ${steps[2].verb}`);
                expect("their email copies itself", steps[3].verb === "copy", steps[3].verb);
                // Nor is anyone's drawer made with them: it's made when it's
                // first shown, and once.
                const eager = people.everyone.filter(row => {
                    const drawer = Object.getOwnPropertyDescriptor(row, "drawer");
                    if (!drawer || !drawer.get)
                        return true;
                    const shown = row.drawer;
                    return row.drawer !== shown;
                });
                expect("no drawer made with everyone", eager.length === 0, names(eager));
            }
        },
        seeking("hal", "one with no email", () => {
            const hal = found("hal");
            expect("found", names(hal) === "Hal", names(hal));
            expect("called by their id", reach(hal[0]) === "tuple call start 108",
                Niri.spawned.join(" "));
            expect("and nothing of theirs to copy",
                names(hal[0].verbs()) === "Call Them, Favourite", names(hal[0].verbs()));
        }),
        seeking("eli", "flipping Favourite", () => {
            const making = found("eli")[0].verbs()[1];
            expect("a stranger isn't a favourite", making.name === "Favourite"
                && making.side === "no" && making.verb === "star", `${making.side} ${making.verb}`);
            making.run();
            expect("telling Tuple of a favourite",
                Niri.spawned.join(" ") === "tuple contacts favorite eli@example.com",
                Niri.spawned.join(" "));
            expect("the switch says so at once", making.side === "yes", making.side);
            expect("a new favourite at home at once",
                names(people.all) === "Ada Brook, Ben Carrow, Dee Ellery, Eli Fenwick, Cal Dunmore",
                names(people.all));
            expect("with a star", home("Eli Fenwick").glyph === String.fromCodePoint(0xf04ce),
                home("Eli Fenwick").glyph);

            home("Dee Ellery").verbs()[1].run();
            expect("telling Tuple of one no longer a favourite",
                Niri.spawned.join(" ") === "tuple contacts unfavorite dee@example.com",
                Niri.spawned.join(" "));
            expect("one no longer a favourite, with nothing else between them, isn't listed",
                names(people.all) === "Ada Brook, Ben Carrow, Eli Fenwick, Cal Dunmore",
                names(people.all));
        }),
        {
            // As Tuple says them now, which never heard of the favourites
            // changed here.
            what: "a favourite gone into a call, as Tuple says it",
            run: () => {
                touching.running = true;
            },
            holds: () => names(people.all) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore"
                && sides(people.all) === "in a call, in a call, away, offline",
            check: () => {
                expect("is joined", people.all[0].verbs()[0].name === "Join Their Call",
                    people.all[0].verbs()[0].name);
            }
        }
    ]

    // The steps in order, each waited on up to 30 s from when it starts,
    // so a machine slowed by other work fails no step that came in time.
    property int step: 0
    property int left: 0

    function begin() {
        const now = steps[step];
        left = 600;
        try {
            if (now.run)
                now.run();
        } catch (error) {
            fail(`${now.what}: ${error.message}`);
            return;
        }
        waiting.start();
    }

    Timer {
        id: waiting

        interval: 50
        repeat: true
        onTriggered: {
            const now = root.steps[root.step];
            if (!now.holds || now.holds()) {
                stop();
                try {
                    if (now.check)
                        now.check();
                } catch (error) {
                    root.fail(`${now.what}: ${error.message}`);
                    return;
                }
                root.step += 1;
                if (root.step === root.steps.length)
                    Qt.callLater(() => Qt.exit(0));
                else
                    root.begin();
            } else {
                root.left -= 1;
                if (root.left === 0) {
                    stop();
                    root.fail(`never came: ${now.what}: home ${root.names(people.all)}`
                        + ` (${root.sides(people.all)}), sought ${people.sought.words}`);
                }
            }
        }
    }

    Component.onCompleted: begin()
}
