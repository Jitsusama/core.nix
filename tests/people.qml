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
// changes how they stand, as Tuple says it, keeping the favourites
// changed here until Tuple says them, and a place holding a row People
// lent it, as a mail its sender, is told to ask for it again. A guest
// asked after before the daemon answers is named once it does, and one
// Tuple comes to know after their name was typed is found without typing
// it again. It exits 0 once every case
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
        // The steps held, as the launcher's are, each with its rows.
        property var held: []
        // Whether People's rows are wanted (Provider.qml's wanted), as
        // the launcher says from where it stands.
        property bool wanting: true

        function wants(source) {
            return wanting;
        }

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

    // A meeting's guest, asked after as Calendar's rows ask (guesting)
    // before the daemon has answered, and how many merges People had
    // once it was named.
    QtObject {
        id: guest

        property bool asking: true
        readonly property var named: asking ? people.whoIs("ada@example.com") : null
        property int merges: 0
    }

    // A place holding a row People lent it, as Mail holds a mail's
    // sender, and how often it was told its rows changed.
    QtObject {
        id: mailer

        signal changed
        property int told: 0

        onChanged: told += 1
    }

    // A place holding a row it serves for someone People doesn't list
    // yet, as Calendar holds a guest before the people daemon has heard
    // from google, and how often it was told its rows changed.
    QtObject {
        id: guesting

        signal changed
        property int told: 0

        onChanged: told += 1
    }

    // Tuple's next presence, which the stand-in waits for.
    Process {
        id: touching
        command: ["touch", `${root.runtime}/next`]
    }

    // Tuple's presence after that, bringing Gil, whom nobody listed.
    Process {
        id: naming
        command: ["touch", `${root.runtime}/again`]
    }

    // The people daemon stopped, as when it fails, and started again.
    Process {
        id: stopping
        command: ["sh", "-c", `kill "$(cat ${root.runtime}/people.pid)"`]
    }
    Process {
        id: starting
        command: [Quickshell.env("PEOPLE"), `${root.runtime}/daemons/people.sock`,
            Quickshell.env("SOURCES"), Quickshell.env("WORDS")]
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

    // What Return does first inside someone: call or join them, after what
    // waits on Joel, which comes before (the stand-in's Lunch on Friday).
    function firstAct(row) {
        return row.verbs().find(one => one.key !== "mail:lunch");
    }
    function reach(row) {
        firstAct(row).run();
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
            // Who someone is was read with nothing to follow, so a guest
            // or a sender drawn before People answered stayed unnamed
            // (Y2YBTS, 2026-11-10), or known only by their address, as a
            // page that names no one knows them.
            what: "a guest asked after before People answers is named once it does",
            holds: () => guest.named?.name === "Ada Brook",
            check: () => {
                guest.merges = people.merging.merges;
                guest.asking = false;
            }
        },
        {
            what: "with nothing typed, Tuple's favourites, then those called lately",
            holds: () => names(people.all) === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore",
            check: () => {
                const open = people.all;
                // Those listed are the daemon's, merged, standing and in
                // order: with 600 people, merging them here took the GUI
                // thread 26 to 38 ms, and ordering them 19 more
                // (2026-11-10).
                expect("listed as the daemon lists them, with no one merged here",
                    people.merging.merges === guest.merges, `${people.merging.merges} merges`);
                // Everything a row and its person hold read, as a whole
                // copy reads it: a getter for their facts there, or for
                // the row's drawer, merged everyone (Y2YBTS, 2026-11-10),
                // so only a drawer shown reads facts.
                for (const held of [open[0], open[0].person])
                    for (const name of Object.getOwnPropertyNames(held))
                        void held[name];
                expect("reading someone listed merges no one",
                    people.merging.merges === guest.merges,
                    `${people.merging.merges} merges`);
                // Ada's time follows, as Kolkata's isn't the UTC here.
                expect("how each stands",
                    /^available \u00b7 Kolkata \u00b7 \d\d:\d\d, in a call, away, offline$/u
                        .test(sides(open)), sides(open));
                expect("a favourite wears a star",
                    open[0].glyph === String.fromCodePoint(0xf04ce), open[0].glyph);
                expect("one called lately doesn't",
                    open[3].glyph === String.fromCodePoint(0xf0004), open[3].glyph);
                expect("no one else is held", people.persons().length === 4,
                    names(people.persons()));
                // Who an address is, for a meeting's guests counted or a
                // call's name, is known with no row made: counting a
                // meeting's guests by their rows made each one's whole row
                // every time its acts were read (2026-11-09).
                const madeBefore = people.tally.rows;
                // Home counts and names those listed from the people
                // themselves, so it never wants their rows: wanting them
                // listed every row again at every opening (2026-11-10).
                expect("home says itself", people.saysAtHome, people.saysAtHome);
                expect("home counts those listed", people.homeSays() === "4",
                    people.homeSays());
                expect("and names them", people.memberNames().join(", ")
                    === "Ada Brook, Ben Carrow, Dee Ellery, Cal Dunmore",
                    people.memberNames().join(", "));
                const ada = people.whoIs(" ADA@example.com ");
                expect("who an address is, whatever its case", ada?.name === "Ada Brook",
                    ada?.name);
                expect("an address nobody knows is no one",
                    people.whoIs("nobody@example.com") === null, "");
                expect("and knowing them makes no row", people.tally.rows === madeBefore,
                    people.tally.rows - madeBefore);
                expect("while their row is still theirs to step into",
                    people.personOf("ada@example.com")?.name === "Ada Brook", "");
                // A guest google's daemon serves lends People's own row
                // (RTOIMQ and Y2YBTS, 2026-11-12), looked up, never asked.
                const lentAda = people.lent("person:ada@example.com");
                expect("a served guest is lent their own row", lentAda?.name === "Ada Brook"
                    && lentAda.component === "person", `${lentAda?.name} ${lentAda?.component}`);
                expect("anyone else lends nothing, so the served row stands",
                    people.lent("person:stranger@example.com") === null
                    && people.lent("gdoc:ada@example.com") === null
                    && people.lent(undefined) === null, "");
                // An address nothing here asked whoIs after, so only lent()
                // could have asked the daemon.
                expect("and lending asks the daemon after no one",
                    !("stranger@example.com" in people.unlisted.by), "");
                // A pick forgets the picks of rows gone by every row's key
                // (Launcher.qml's knowing), asked of People wanted or not:
                // read as rows, it made every listed person's row while
                // People wasn't wanted, 116 at one of Joel's picks
                // (2026-11-10).
                launcher.wanting = false;
                const unwanted = people.tally.rows;
                const keys = people.keysKnown().map(one => one.key).join(", ");
                expect("a pick knows who's listed with no row made while unwanted",
                    people.tally.rows === unwanted, people.tally.rows - unwanted);
                launcher.wanting = true;
                expect("by the keys their rows have", keys === open.map(row => row.key)
                    .join(", "), keys);
            }
        },
        {
            // Someone the daemon holds but doesn't list, as a guest seen
            // only at a meeting still to come, was no one once Google's
            // people were the daemon's rather than merged here, so their
            // meeting counted them out.
            what: "one the daemon holds and doesn't list is known by their address once asked",
            run: () => {
                root.unlistedFirst = people.whoIs("zoe@example.com");
            },
            holds: () => people.whoIs("zoe@example.com") !== null,
            check: () => {
                expect("no one until the daemon answers", root.unlistedFirst === null,
                    root.unlistedFirst?.name);
                const zoe = people.whoIs(" Zoe@Example.com ");
                expect("named as the daemon holds them", zoe?.name === "Zo\u00eb \"Zed\" Quill"
                    && zoe.key === "zoe@example.com", `${zoe?.name} ${zoe?.key}`);
                expect("and still not listed, nor held with those listed",
                    people.persons().length === 4, names(people.persons()));
                expect("and lent once the daemon has named them",
                    people.lent("person:zoe@example.com")?.name === "Zo\u00eb \"Zed\" Quill",
                    people.lent("person:zoe@example.com")?.name);
            }
        },
        {
            // Their drawer is the one the daemon serves, their title under
            // their name, as a search's is: the one the shell worded
            // itself knew only their address (XY9H5C, 2026-11-12).
            what: "one named only by their address is drawn as the daemon serves them",
            run: () => people.whoIs("eli@example.com"),
            holds: () => people.whoIs("eli@example.com") !== null,
            check: () => {
                const drawn = people.lent("person:eli@example.com")?.drawer?.();
                expect("their title under their name, and not among the facts",
                    drawn?.title === "Eli Fenwick"
                        && JSON.stringify(drawn.preview?.lines) === '["Designer"]'
                        && !drawn.facts.some(fact => fact.label === "Title"),
                    JSON.stringify(drawn));
            }
        },
        seeking("eli@example.com", "a stranger found by their email", () => {
            const eli = found("eli@example.com");
            expect("found", names(eli) === "Eli Fenwick", names(eli));
            expect("who stands unknown says nothing", eli[0].side === "", eli[0].side);
        }),
        seeking("gus", "one with only an email, named from it", () => {
            const gus = found("gus");
            expect("named from it, and saying it under", names(gus) === "Gus"
                && gus[0].note === "gus@example.com" && gus[0].side === "",
                `${names(gus)} ${gus[0]?.note} ${gus[0]?.side}`);
        }),
        seeking("quill", "a name with a quote and more than ASCII", () => {
            const zoe = found("quill");
            expect("found", names(zoe) === "Zo\u00eb \"Zed\" Quill", names(zoe));
        }),
        seeking("ada", "one listed, found as they stand", () => {
            const ada = found("ada");
            // Here is UTC, so her Kolkata time, five and a half hours on,
            // follows how she stands (VKCF3Z, 0cea8a93: "London · 17:02").
            expect("as they stand, and their time where it's apart from his",
                names(ada) === "Ada Brook"
                    && /^available \u00b7 Kolkata \u00b7 \d\d:\d\d$/u.test(ada[0].side),
                `${names(ada)} ${ada[0]?.side}`);
        }),
        // Among equals, the daemon's order: those found by a name before
        // those found by an address alone, as gus is, then
        // as Tuple lists them.
        seeking("example", "those listed first, then the rest, as the daemon finds them", () => {
            const all = found("example");
            expect("in order", names(all) === "Ada Brook, Ben Carrow, Cal Dunmore, Dee Ellery, "
                + "Gus, Eli Fenwick, Zo\u00eb \"Zed\" Quill", names(all));
        }),
        seeking("ben", "the shell holds only those listed and those found", () => {
            expect("only those", people.persons().length === 4, names(people.persons()));
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
                expect("joining one in a call",
                    reach(open[1]) === "tuple call join ben@example.com", Niri.spawned.join(" "));
                expect("calling one who's offline",
                    reach(open[3]) === "tuple call start cal@example.com", Niri.spawned.join(" "));

                const steps = open[0].verbs();
                expect("their page: what waits first, then the ways to reach them, what's "
                    + "between them, the address last", names(steps) === "Lunch on Friday, "
                    + "Call Them, Open in Pages, Last Met, Messages, Favourite, ada@example.com",
                    names(steps));
                // What's between them, as VKCF3Z ruled it (0cea8a93), and
                // as a show of two people for it: one with mail waiting,
                // one without.
                const shown = rows => rows.map(row => `${row.name}  [${row.side ?? ""}]`
                    + (row.heldNames ? ` holding ${row.heldNames.join("; ")}` : ""));
                console.log(`Ada's page:\n  ${shown(steps).join("\n  ")}`);
                console.log(`Ben's page:\n  ${shown(open[1].verbs()).join("\n  ")}`);
                expect("one thing waiting stands as itself, Return its own act, saying why",
                    steps[0].key === "mail:lunch"
                        && steps[0].side.startsWith("needs you \u00b7 mail \u00b7 "),
                    `${steps[0].key} ${steps[0].side}`);
                expect("Last Met says when the newest was",
                    steps[3].side === "yesterday" && steps[3].verbs().length === 2,
                    `${steps[3].side} ${names(steps[3].verbs())}`);
                expect("Messages says who wrote last and when, leaving out what waits",
                    steps[4].side === "you wrote \u00b7 5 min ago",
                    `${steps[4].side} ${names(steps[4].verbs())}`);
                expect("and holds the rest, newest first",
                    names(steps[4].verbs()) === "See you then, Trip Notes",
                    names(steps[4].verbs()));
                const ben = open[1].verbs();
                const benMet = ben.find(row => row.name === "Last Met");
                expect("nothing waiting, no Waiting; a meeting of the two of them says so",
                    !names(ben).includes("Waiting") && !names(ben).includes("Lunch")
                        && benMet?.side === "yesterday \u00b7 1:1",
                    `${names(ben)} ${benMet?.side}`);
                const thanks = ben.find(row => row.name === "Thanks!");
                expect("one message alone stands as itself, after Last Met, saying who wrote",
                    names(ben).includes("Last Met, Thanks!, Favourite")
                        && thanks?.side === "they wrote \u00b7 yesterday",
                    `${names(ben)} ${thanks?.side}`);
                steps[2].run();
                expect("their page opens", steps[2].verb === "open" && Niri.spawned.join(" ")
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
                // Their drawer is the daemon's, a source's facts ready in
                // it, so showing it merges no one.
                const merges = people.merging.merges;
                const told = open[0].drawer().facts.map(fact => `${fact.label}: ${fact.value}`);
                // The first of each comes first, what needs him ahead
                // (VKCF3Z, 0cea8a93): her mail unread, and the last they met.
                console.log(`Ada's drawer: ${told.join("; ")}`);
                expect("their drawer opens on the first of each",
                    told[0] === "Waiting: Lunch on Friday \u00b7 1 min ago"
                        && told[1] === "Last Met: Retro \u00b7 yesterday",
                    told.slice(0, 2).join("; "));
                const facts = told.slice(2);
                expect("a source's facts are theirs, the time there after their zone",
                    facts[0] === "Email: ada@example.com"
                        && facts[1] === "Time Zone: Asia/Kolkata"
                        && /^Time There: \d\d:\d\d, 5 h 30 min ahead$/.test(facts[2]),
                    facts.join("; "));
                // Her team is said under her name instead, once (VKCF3Z, a807ddcd).
                const head = open[0].drawer().preview;
                expect("their team under their name, quiet, and not among the facts",
                    JSON.stringify(head?.lines) === '["Shell"]' && head.quiet && head.gives
                        && !facts.some(fact => fact.startsWith("Team:")),
                    JSON.stringify(head));
                expect("showing their drawer merges no one", people.merging.merges === merges,
                    `${people.merging.merges - merges} merges`);
                expect("a favourite says so", steps[5].side === "yes" && steps[5].verb === "unstar",
                    `${steps[5].side} ${steps[5].verb}`);
                expect("their email copies itself", steps[6].verb === "copy", steps[6].verb);
                // Nor is anyone's drawer made with them: it's made when it's
                // first shown, and kept for the minute its time there holds.
                const eager = people.everyone().filter(row => {
                    if (typeof row.drawer !== "function")
                        return true;
                    const shown = row.drawer();
                    return row.drawer() !== shown;
                });
                expect("no drawer made with everyone", eager.length === 0, names(eager));
                // A person's insides ask every source, so Ctrl+]'s look for
                // where a row lives never goes inside one (Catalogue.js's
                // followsInto), which a person's row says by being parted.
                const whole = people.everyone().filter(row => row.parted !== true);
                expect("everyone's row is parted, so no look makes their insides",
                    whole.length === 0, names(whole));
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
            const making = found("eli")[0].verbs().find(row => row.name === "Favourite");
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

            home("Dee Ellery").verbs().find(row => row.name === "Favourite").run();
            expect("telling Tuple of one no longer a favourite",
                Niri.spawned.join(" ") === "tuple contacts unfavorite dee@example.com",
                Niri.spawned.join(" "));
            expect("one no longer a favourite, with nothing else between them, isn't listed",
                names(people.all) === "Ada Brook, Ben Carrow, Eli Fenwick, Cal Dunmore",
                names(people.all));
        }),
        // Words that find Eli too, whose favourite changed here is kept
        // while the search holds his record.
        seeking("f", "one Tuple doesn't know yet isn't found", () => {
            expect("not found", !names(found("f")).includes("Fay Gale"), names(found("f")));
            root.foundBefore = root.foundMore;
        }),
        {
            // Tuple's change heard while People's rows aren't wanted, as
            // with the launcher shut: merging everyone at every change
            // took 26 to 38 ms of the GUI thread with nobody looking
            // (2026-11-10), so nobody is merged until someone asks.
            what: "a change heard while nobody looks",
            run: () => {
                launcher.wanting = false;
                // Ada's row lent to a mail shown, which said how she stood
                // as it was made, and stayed so (Y2YBTS, 2026-11-10).
                launcher.held = [{ row: { key: "mail:1", from: mailer },
                    rows: [people.personOf("ada@example.com")] }];
                root.mergedBefore = people.merging.merges;
                root.heardBefore = people.revision;
                touching.running = true;
            },
            holds: () => people.revision > root.heardBefore,
            check: () => {
                const merges = people.merging.merges;
                expect("merges no one", typeof merges === "number" && merges === root.mergedBefore,
                    `${merges} merges, ${root.mergedBefore} before`);
                expect("tells the place a row was lent to, to ask for it again",
                    mailer.told > 0, `told ${mailer.told} times`);
                launcher.held = [];
                // Warmed shut as the shell starts, those listed are made
                // then, so the first letter typed makes none of them: made
                // by it, 121 rows held Joel's 15 ms (2026-11-11 23:10).
                people.warming = true;
                people.warming = false;
                const warmed = people.tally.rows;
                launcher.wanting = true;
                void people.all;
                expect("once warmed, wanting those listed makes no row",
                    people.tally.rows === warmed, `${people.tally.rows - warmed} rows made`);
            }
        },
        {
            // Google's people came 14 s after a name was typed, and the
            // person never showed until the words changed (80ECO2,
            // 2026-11-11, vm-desktop-looks-plates).
            what: "one Tuple comes to know after the words were typed is found, untyped again",
            holds: () => root.foundMore > root.foundBefore,
            check: () => {
                expect("found", names(found("f")).includes("Fay Gale"), names(found("f")));
            }
        },
        {
            // As Tuple says them now, with the favourites changed here,
            // which the stand-in never says, kept as they were asked.
            what: "a favourite gone into a call, as Tuple says it",
            holds: () => names(people.all) === "Ada Brook, Ben Carrow, Eli Fenwick, Cal Dunmore"
                && sides(people.all) === "in a call, in a call, , offline",
            check: () => {
                expect("is joined", firstAct(people.all[0]).name === "Join Their Call",
                    firstAct(people.all[0]).name);
            }
        },
        {
            // A down daemon is said at home rather than a count of whom it
            // last listed, which would read as if all were well.
            what: "the people daemon gone",
            run: () => {
                root.stoppedAt = Date.now();
                stopping.running = true;
            },
            holds: () => people.homeSays() === "not answering",
            check: () => {
                // Only once silent a second, so a restart or a reconnect
                // never blinks it (VKCF3Z, 2026-11-10).
                const silent = Date.now() - root.stoppedAt;
                expect("said only once silent a second", silent >= 1000, `${silent} ms`);
                expect("still lists whom it last listed", people.all.length === 4,
                    names(people.all));
                // Said inside People too, on its head, as home's row says it.
                expect("said on People's head", people.pressing === "not answering",
                    people.pressing);
            }
        },
        {
            what: "the people daemon back",
            run: () => {
                starting.running = true;
            },
            holds: () => people.pressing === "",
            check: () => {
                expect("home counts again", people.homeSays() !== "not answering",
                    people.homeSays());
            }
        },
        {
            // Gil, served as a guest before anyone lists him, whose row
            // People can lend only once a source lists him: Guests held
            // while google's people came stayed served rows until the
            // launcher shut (RTOIMQ, 2026-11-12), since only a lent row
            // shown was told of a change.
            what: "one newly listed while a place holds them unlent",
            run: () => {
                launcher.held = [{ row: { key: "calendar:meeting:standup", from: guesting },
                    rows: [{ key: "calendar:meeting:standup:guest:gil@example.com",
                        lends: "person:gil@example.com", name: "gil@example.com" }] }];
                guesting.told = 0;
                naming.running = true;
            },
            holds: () => names(people.all).includes("Gil Hart"),
            check: () => {
                launcher.held = [];
                expect("tells the place holding them, to ask for them again",
                    guesting.told > 0, `told ${guesting.told} times`);
            }
        }
    ]

    // When the people daemon was stopped.
    property real stoppedAt: 0

    // How often People found more once asked, as the launcher hears it,
    // and how often before Tuple came to know Fay.
    property int foundMore: 0
    property int foundBefore: 0
    Connections {
        target: people
        function onFoundMore() {
            root.foundMore += 1;
        }
    }

    // How many merges and changes People had before a change heard while
    // nobody looks.
    property var mergedBefore: null
    // Who an unlisted address was the moment it was first asked after.
    property var unlistedFirst: undefined
    property int heardBefore: 0

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
