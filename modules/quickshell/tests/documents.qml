// Drives the launcher's Documents (modules/google/Documents.qml) through
// Quickshell itself, with the real ReadyRows.qml making each row from the
// one the google daemon serves ready, and Google's account, the mail and
// the saves stood in. Its daemon is told records as the google daemon
// tells them (src/document_rows.rs), through ReadyRows' own Daemon.qml's
// took, with no daemon to answer: every case runs in one turn, before a
// refused connection could let them go. With the launcher shut, rows
// told and the clock's turn make no row, though each row says when it
// was opened against the clock; opened, it makes them once, in the
// daemon's order; opened again, it makes only those that changed; a row
// told ahead of the rest goes first, and one gone goes; and shut again,
// it keeps the rows it made. A copy of a row makes no drawer, and its
// drawer reads its record as it is now. Another place serving a document
// as a row of its own, lending it, shows Documents' row for it, or its
// own while Documents holds none, and only that row is made again as
// Documents' change. What's typed inside a step is asked of its daemon
// within it, and its answer's rows held beside the watch's, the same key
// the same row, and one not answered is said as none. A search's answer
// to words typed over is held, and one asked before the search ended is
// let go, what's typed after asked anew. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // A document's ready row, Joel's, opened an hour ago, at `order`.
    function row(id, title, order) {
        const key = `row:documents:${id}`;
        const link = `https://docs.google.com/document/d/${id}/edit`;
        const viewed = Date.now() - 3600000;
        return { key, kind: "row", row: {
            key, thing: `gdoc:${id}`, place: "documents", kind: "document", name: title,
            glyph: "googleDocument", hue: "program", depth: 0,
            side: [{ at: viewed, as: "opened", keep: 1 }], drawer: `drawer:documents:${id}`,
            acts: [{ key: "verb:docs:open", open: link, in: "google-opens" },
                { key: "verb:docs:link", copy: link }],
            held: { link, viewed, mime: "" }, order
        } };
    }

    function drawer(id, title) {
        const key = `drawer:documents:${id}`;
        return { key, kind: "drawer", drawer: { eyebrow: "in My Drive \u00b7 Document", title,
            facts: [{ name: "Owner", said: "Ada Lovelace" }], body: "" } };
    }

    function tell(records, gone) {
        documents.served.link.took(records, gone ?? []);
    }

    QtObject {
        id: standIn

        property bool open: false
        // The places the launcher holds, as Launcher.qml's sources, so a
        // place serving a document as its own row borrows Documents'.
        property var sources: [documents]
        // Whether Documents' rows are wanted (Provider.qml's wanted), as
        // the launcher says from where it stands: never while it's shut.
        function wants(source) {
            return open;
        }
    }

    Launcher.Documents {
        id: documents

        launcher: standIn
        settings: ({ name: "documents" })
    }

    // Another place's rows, as a meeting's attachments are served, each
    // made as served (ReadyRows.qml's make) unless a place lends it.
    Launcher.ReadyRows {
        id: borrower

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/borrower.sock`
        place: "calendar"
        maker: QtObject {
            readonly property var launcher: standIn
        }
    }

    // A row of the borrower's at `order`, lending the document `id`, or
    // with `id` empty, one of its own.
    function borrowed(name, id, order) {
        const key = `row:calendar:${name}`;
        const lends = id === "" ? {} : { thing: `gdoc:${id}`, lends: `gdoc:${id}` };
        return { key, kind: "row", row: Object.assign({
            key, place: "calendar", kind: "attachment", name, glyph: "", hue: "program",
            depth: 0, side: [], acts: [], order
        }, lends) };
    }

    function titles() {
        return documents.things.map(made => made.name).join(", ");
    }

    // The cases run once Quickshell has read the entry each row opens
    // through (google-opens, the act's `in`), which it may read after the
    // shell starts, since a row whose entry is gone offers no open.
    property bool begun: false
    Component.onCompleted: begin()
    Connections {
        target: DesktopEntries.applications
        function onValuesChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || DesktopEntries.byId("google-opens") === null)
            return;
        begun = true;
        try {
            const shut = documents.tally.rows;
            tell([row("d1", "Quarterly Plan", 1024), drawer("d1", "Quarterly Plan")]);
            Expect.expect("shut, rows told make no row", documents.tally.rows === shut,
                documents.tally.rows - shut);
            // Every row says when it was opened, as minutes ago, so the
            // clock's minute remade them all with the launcher shut, 27 a
            // minute on Joel's shell (2026-11-10).
            documents.served.turned();
            tell([row("d2", "Roadmap", 2048), drawer("d2", "Roadmap")]);
            Expect.expect("nor the clock turning and another told", documents.tally.rows === shut,
                documents.tally.rows - shut);
            standIn.open = true;
            Expect.expect("opened, it holds every document in the daemon's order",
                titles() === "Quarterly Plan, Roadmap", titles());
            Expect.expect("made once", documents.tally.rows - shut === 2, documents.tally.rows - shut);
            Expect.expect("each the thing it was, opening its link",
                documents.things[0].key === "gdoc:d1"
                && documents.things[0].verb === Words.verb("verb:docs:open"),
                JSON.stringify([documents.things[0].key, documents.things[0].verb]));
            Expect.expect("and its program's Recent holds them",
                documents.recentOf("google-docs").map(made => made.name).join(", ")
                === "Quarterly Plan, Roadmap", "");
            // Opened again, it makes only what changed: every opening made
            // all 27 of Joel's again (2026-11-10).
            standIn.open = false;
            const opened = documents.tally.rows;
            standIn.open = true;
            Expect.expect("opened again, it makes none that reads the same",
                documents.tally.rows === opened, documents.tally.rows - opened);
            tell([row("d2", "Roadmap, Revised", 2048), drawer("d2", "Roadmap, Revised")]);
            Expect.expect("and one changed, only it", documents.tally.rows - opened === 1,
                documents.tally.rows - opened);
            Expect.expect("as it reads now", titles() === "Quarterly Plan, Roadmap, Revised",
                titles());
            const ahead = documents.tally.rows;
            tell([row("d3", "Archive", 512)]);
            Expect.expect("one told ahead goes first",
                titles() === "Archive, Quarterly Plan, Roadmap, Revised", titles());
            Expect.expect("made alone", documents.tally.rows - ahead === 1,
                documents.tally.rows - ahead);
            tell([], ["row:documents:d3"]);
            Expect.expect("and gone, goes", titles() === "Quarterly Plan, Roadmap, Revised",
                titles());
            // A copy of a row reads all it holds, so a drawer it held as
            // a getter would be made with every copy (Y2YBTS, 2026-11-10).
            const copied = documents.things.map(made => Object.assign({}, made));
            Expect.expect("a copy of every row makes no drawer",
                copied.every(made => typeof made.drawer === "function"),
                copied.map(made => typeof made.drawer).join(", "));
            const shown = documents.things[1].drawer();
            Expect.expect("its drawer as it reads now",
                shown?.title === "Roadmap, Revised"
                && shown.facts[0].value === "Ada Lovelace", JSON.stringify(shown));
            // A document another place serves is Documents' own row, its
            // drawer with it, and one Documents doesn't hold stands as
            // served, until Documents holds it (Y2YBTS, 2026-11-12).
            borrower.link.took([borrowed("plan", "d1", 1024), borrowed("late", "d9", 2048),
                borrowed("own", "", 3072)], []);
            const lentPlan = borrower.rowAt("row:calendar:plan");
            Expect.expect("a row lent is its lender's own",
                lentPlan === documents.things.find(made => made.key === "gdoc:d1"),
                JSON.stringify([lentPlan?.key, lentPlan?.name]));
            Expect.expect("one nobody lends stands as served",
                borrower.rowAt("row:calendar:late")?.name === "late",
                borrower.rowAt("row:calendar:late")?.name);
            const own = borrower.rowAt("row:calendar:own");
            tell([row("d9", "Late Plan", 4096)]);
            Expect.expect("a lender arriving late lends it",
                borrower.rowAt("row:calendar:late")?.key === "gdoc:d9",
                borrower.rowAt("row:calendar:late")?.key);
            Expect.expect("and the rows it lends nothing are kept as made",
                borrower.rowAt("row:calendar:own") === own, "");
            tell([], ["row:documents:d9"]);
            Expect.expect("and gone, the served row stands again",
                borrower.rowAt("row:calendar:late")?.name === "late",
                borrower.rowAt("row:calendar:late")?.name);
            // A row that stands only where it's lent is none where nobody
            // lends it, so a meeting offers no Open the Transcript for a
            // call nobody wrote down, nor on a machine without
            // call-transcripts (Y2YBTS's 2b6b2cc5).
            const inMeeting = (name, id, order) => {
                const made = borrowed(name, id, order);
                Object.assign(made.row, { parent: "row:calendar:meet", depth: 1 },
                    id === "" ? {} : { lentOnly: true });
                return made;
            };
            borrower.link.took([borrowed("meet", "", 5120), inMeeting("opened", "d1", 6144),
                inMeeting("unwritten", "d8", 7168), inMeeting("guests", "", 8192)], []);
            const inside = () => borrower.rows("row:calendar:meet").map(made => made.name)
                .join(", ");
            Expect.expect("one standing only where lent is its lender's row",
                borrower.rowAt("row:calendar:opened")?.key === "gdoc:d1",
                borrower.rowAt("row:calendar:opened")?.key);
            Expect.expect("and none where nobody lends it, the row it's in holding one less",
                borrower.rowAt("row:calendar:unwritten") === null
                && inside() === "Quarterly Plan, guests", inside());
            tell([row("d8", "Meeting Notes", 8192)]);
            Expect.expect("until a lender holds it",
                inside() === "Quarterly Plan, Meeting Notes, guests", inside());
            tell([], ["row:documents:d8"]);
            Expect.expect("and none again once it's gone", inside() === "Quarterly Plan, guests",
                inside());
            // What's typed inside a step is asked of its daemon within the
            // step, and the rows answered held beside the watch's, the
            // same key the same row, so none moves under the cursor; one
            // not answered is none, for the place to say so (Y2YBTS's
            // bf11aae8).
            const move = "row:calendar:plan:move";
            const asking = borrower.link.asked;
            Expect.expect("with no daemon to ask, what's typed is said unanswered at once",
                borrower.typedIn(move, "thu 2") === null && borrower.link.asked === asking + 1,
                borrower.link.asked - asking);
            Expect.expect("and isn't asked again for the same words",
                borrower.typedIn(move, "thu 2") === null && borrower.link.asked === asking + 1,
                borrower.link.asked - asking);
            const time = `${move}:2026-11-19T19:00:00Z`;
            const timed = (key, parent, name) => ({ key, kind: "row", row: { key, parent, name,
                place: "calendar", kind: "time", depth: 2, glyph: "", hue: "action", side: [],
                acts: [] } });
            const answer = words => ({ id: "found", words, keys: [time], rows: {
                [time]: timed(time, move, "Thursday"),
                [`${time}:sure`]: timed(`${time}:sure`, time, "Move It") } });
            // As a daemon that answers would be asked.
            borrower.typedAsking = `${move}\nthu 2pm`;
            borrower.typedAnswered(move, "thu 2", answer("thu 2"));
            Expect.expect("an answer to words typed over is let go, the last one standing",
                borrower.typedIn(move, "thu 2pm") === null, "");
            borrower.typedAnswered(move, "thu 2pm", answer("thu 2pm"));
            const typed = borrower.typedIn(move, "thu 2pm");
            Expect.expect("what's typed is the daemon's rows",
                typed?.map(made => made.name).join(", ") === "Thursday", JSON.stringify(typed));
            Expect.expect("and inside one, the row it holds",
                borrower.rows(time).map(made => made.name).join(", ") === "Move It", "");
            Expect.expect("and none of them inside the step itself",
                borrower.rows(move).length === 0, borrower.rows(move).length);
            borrower.typedAsking = `${move}\nthu 2 pm`;
            borrower.typedAnswered(move, "thu 2 pm", answer("thu 2 pm"));
            Expect.expect("the same time typed on is the same row",
                borrower.typedIn(move, "thu 2 pm")?.[0] === typed[0], "");
            borrower.typedAsking = `${move}\nthu 2 pm,`;
            Expect.expect("and stays while what's typed on is asked",
                borrower.typedIn(move, "thu 2 pm,")?.[0] === typed[0], "");
            borrower.typedAsking = `${move}\nsoon`;
            borrower.typedAnswered(move, "soon", { id: "no",
                error: { code: "unknown-ask", message: "no" } });
            Expect.expect("one refused is none", borrower.typedIn(move, "soon") === null, "");
            Expect.expect("and its rows let go", borrower.rows(time).length === 0, "");
            // A search's answer to words typed over is held, as Emoji and
            // Code show it while what's typed on is asked; one asked
            // before the search ended, as the launcher shutting ends it,
            // is let go, or it stood at the next opening for what's typed
            // on from it. What's typed after is asked anew. Each is
            // answered as the daemon would, through what the link awaits,
            // so what search() kept when it asked is what's weighed.
            const link = borrower.link;
            link.wire = { write: line => {}, flush: () => {} };
            const asked = words => {
                borrower.search(words);
                const id = `find-${link.asked}`;
                return () => link.awaited[id]({ id, words, keys: ["row:calendar:own"] });
            };
            const typedOver = asked("pla");
            const late = asked("plan");
            typedOver();
            Expect.expect("a search's answer to words typed over is held, still asking",
                borrower.found.words === "pla" && borrower.asking === "plan",
                JSON.stringify([borrower.found.words, borrower.asking]));
            borrower.ended();
            const searching = link.asked;
            const anew = asked("plan");
            Expect.expect("ended, what's typed after is asked anew",
                link.asked === searching + 1, link.asked - searching);
            late();
            Expect.expect("and an answer asked before it ended is let go, the live ask still asked",
                borrower.found.words === null && borrower.asking === "plan",
                JSON.stringify([borrower.found.words, borrower.asking]));
            anew();
            Expect.expect("while the one asked after is taken",
                borrower.found.words === "plan", borrower.found.words);
            borrower.ended();
            link.wire = null;
            // Ended, what was typed inside a row is asked anew too, and its
            // answer still to come let go.
            link.wire = { write: line => {}, flush: () => {} };
            borrower.typedIn(move, "lunch");
            const inside = `find-${link.asked}`;
            borrower.ended();
            link.awaited[inside]({ id: inside, keys: [], rows: {} });
            Expect.expect("ended, an answer to what was typed inside a row is let go",
                borrower.typing.words === null && borrower.typedAsking === "",
                JSON.stringify([borrower.typing.words, borrower.typedAsking]));
            link.wire = null;
            // One the daemon couldn't answer, gone or not yet there, is
            // asked again once it says hello, since nothing else asks it
            // till the next key, while its rows are drawn; one whose search
            // ended or whose words were typed over since isn't. Its hello
            // says the daemon stands as it did, so no later case finds it
            // changed.
            borrower.drawn = true;
            const stood = { state: link.state, sources: link.sources, offered: link.offered };
            const hello = () => link.said(JSON.stringify(
                { id: "hello", state: stood.state, sources: stood.sources, kinds: stood.offered }));
            borrower.search("road");
            Expect.expect("asked of no daemon, it's answered with nothing",
                borrower.asking === "" && borrower.found.words === null,
                JSON.stringify([borrower.asking, borrower.found.words]));
            link.wire = { write: line => {}, flush: () => {} };
            const greeted = link.asked;
            hello();
            Expect.expect("and is asked again once the daemon says hello",
                link.asked === greeted + 1 && borrower.asking === "road",
                JSON.stringify([link.asked - greeted, borrower.asking]));
            const again = `find-${link.asked}`;
            link.awaited[again]({ id: again, words: "road", keys: ["row:calendar:own"] });
            Expect.expect("its answer taken", borrower.found.words === "road",
                borrower.found.words);
            const unasked = (what, set) => {
                link.wire = null;
                set();
                link.wire = { write: line => {}, flush: () => {} };
                const before = link.asked;
                hello();
                Expect.expect(what, link.asked === before, link.asked - before);
            };
            unasked("one typed back from to words whose answer is held isn't asked again", () => {
                borrower.search("roads");
                borrower.search("road");
            });
            unasked("one whose search ended isn't asked again", () => {
                borrower.search("rows");
                borrower.ended();
            });
            unasked("one whose rows went undrawn isn't asked again", () => {
                borrower.search("rows");
                borrower.drawn = false;
            });
            unasked("nor one lost while they're undrawn", () => borrower.search("rides"));
            // A connection that goes with the search asked loses it to be
            // asked again, and a line that isn't JSON is said once a
            // connection.
            borrower.drawn = true;
            borrower.search("ride");
            link.gone();
            Expect.expect("a connection gone with the search asked, it waits for the next",
                borrower.asking === "" && borrower.searchState().lost === "ride",
                JSON.stringify(borrower.searchState()));
            link.said("{\"id\": \"find-");
            Expect.expect("a line that isn't JSON is said", link.unreadSaid, "");
            link.wire = { write: line => {}, flush: () => {} };
            const regreeted = link.asked;
            hello();
            Expect.expect("and the search is asked once it's back",
                link.asked === regreeted + 1 && borrower.asking === "ride",
                JSON.stringify([link.asked - regreeted, borrower.asking]));
            link.gone();
            Expect.expect("gone again, the next line that isn't JSON is said too",
                !link.unreadSaid, "");
            borrower.ended();
            borrower.drawn = false;
            link.wire = null;
            standIn.open = false;
            const closed = documents.tally.rows;
            documents.served.turned();
            Expect.expect("shut again, it keeps what it made",
                titles() === "Quarterly Plan, Roadmap, Revised", titles());
            Expect.expect("and makes none", documents.tally.rows === closed,
                documents.tally.rows - closed);
            Qt.callLater(() => Qt.exit(0));
        } catch (failed) {
            Expect.fail(failed.message);
        }
    }
}
