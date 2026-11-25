// Start New Work, Agents' act that begins new work from a link or words, run
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `start-new-work-asks-only-a-sprout` check copies
// modules/quickshell/launcher/Sprouting.js and Openings.js, tests/lib/Expect.js
// and Agents' words, as words.json, beside this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Openings.js" as Openings
import "launcher/Sprouting.js" as Sprouting

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    readonly property var said: ({
        started: "started on {machine}",
        unheld: "started on {machine}; in Agents once up",
        unanswered: "its node didn't answer",
        unread: "its node's answer couldn't be read",
        empty: "say what the work is",
        unplaced: "that folder isn't a whole path",
        actRepository: "start it In a repository",
        askedWhere: "pick where it starts below; your words go with it",
        actPlaced: "once one can take it, say it again",
        actNode: "once the node runs, say it again",
        offered: "a page offered this work; read it to its end, then Return starts it",
        offeredFrom: "a page offered this work, saying it's from {cell}; read it to its end, then Return starts it"
    })

    // What Start New Work says, as Agents' own words, which the check copies
    // beside this file as words.json, hold it.
    function writtenSaid() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const thing = JSON.parse(request.responseText).things
            .find(one => one.id === "agents:start-new-work");
        return thing?.said ?? {};
    }

    function cases() {
        // Only Joel's own Return sprouts, failing closed.
        Expect.same("his Return in the launcher sprouts",
            Sprouting.fromJoel({ cataloguing: false }), true);
        Expect.same("a walk from outside starts nothing",
            Sprouting.fromJoel({ cataloguing: true }), false);
        Expect.same("no launcher starts nothing", Sprouting.fromJoel(null), false);
        Expect.same("an unset launcher starts nothing", Sprouting.fromJoel(undefined), false);
        Expect.same("a launcher that doesn't say it isn't walked starts nothing",
            Sprouting.fromJoel({}), false);

        // What it asks the agents daemon.
        Expect.same("a link alone is sprouted where its repository is checked out",
            Sprouting.ask("https://github.com/owner/repo/pull/7", ""),
            { ask: "sprout", text: "https://github.com/owner/repo/pull/7" });
        Expect.same("words are sprouted in the repository they're said in, as one",
            Sprouting.ask("  tidy the launcher's rows ", "/home/j/src/core"),
            { ask: "sprout", text: "tidy the launcher's rows", folder: "/home/j/src/core" });
        Expect.same("words opening with a dash are words",
            Sprouting.ask("--join now", "/home/j/src/core"),
            { ask: "sprout", text: "--join now", folder: "/home/j/src/core" });
        Expect.same("nothing said starts nothing", Sprouting.ask("   ", "/home/j/src/core"), null);
        Expect.same("a folder that isn't a whole path is never named",
            Sprouting.ask("tidy it", "src/core"), null);

        // New work a mycelium: link offers, as mycelium-link hands it.
        const sessionNamed = address => address === "TERN02" ? "Tidy the Rows" : null;
        Expect.same("a link's words and its session here are offered",
            Sprouting.offered("words:tidy the rows & more", "from:TERN02", sessionNamed),
            { words: "tidy the rows & more", from: { address: "TERN02", name: "Tidy the Rows" } });
        Expect.same("a link from no session offers its words alone",
            Sprouting.offered("words:tidy the rows", "from:", sessionNamed),
            { words: "tidy the rows", from: null });
        for (const from of ["from:GONE42", "from:tern02", "TERN02", "from:TERN02;rm"])
            Expect.same(`a session not here, or misshaped, is let go: ${from}`,
                Sprouting.offered("words:tidy", from, sessionNamed),
                { words: "tidy", from: null });
        // Its own check, with a session here for every address: a misshaped cell names none.
        const anyNamed = () => "Any Quest";
        for (const from of ["from:tern02", "TERN02", "from:TERN02;rm", "from:ABCDEFGHIJKLM"])
            Expect.same(`a misshaped cell names no session, whoever is here: ${from}`,
                Sprouting.offered("words:tidy", from, anyNamed),
                { words: "tidy", from: null });
        Expect.same("words are read again as one plain line",
            Sprouting.offered("words:  tidy\tthe\nrows\u0007\u001b[2J ", "from:", sessionNamed),
            { words: "tidy the rows[2J", from: null });
        Expect.same("words lose what turns, hides or joins text, as mycelium-link's do",
            Sprouting.offered("words:tidy\u202e\u200bthe\u007f\u009brows", "from:", sessionNamed),
            { words: "tidytherows", from: null });
        Expect.same("a byte-order mark joins its words, a next line parts them",
            Sprouting.offered("words:\ufefftidy\ufeffthe\u0085rows\u{e0001}", "from:", sessionNamed),
            { words: "tidythe rows", from: null });
        Expect.same("words holding only format characters offer nothing",
            Sprouting.offered("words:\u202e\u200b", "from:", sessionNamed), null);
        // A run of blanks would hide words between the start and the end.
        const hidden = "\u2800\u3164\u115f\uffa0\ufe0f\u{e0100}".repeat(40);
        Expect.same("words lose what draws a blank, as mycelium-link's do",
            Sprouting.offered(`words:tidy${hidden} the${hidden}rows`, "from:", sessionNamed),
            { words: "tidy therows", from: null });
        Expect.same("words holding only blanks offer nothing",
            Sprouting.offered(`words:${hidden}`, "from:", sessionNamed), null);
        Expect.same("the longest words are offered whole",
            Sprouting.offered(`words:${"\u{1F331}".repeat(1000)}`, "from:", sessionNamed)?.words,
            "\u{1F331}".repeat(1000));
        Expect.same("words past the longest offer nothing, counted by characters",
            Sprouting.offered(`words:${"\u{1F331}".repeat(1001)}`, "from:", sessionNamed), null);
        for (const words of ["tidy", "words:", "words: \t\n", `words:${"a".repeat(1001)}`])
            Expect.same(`no plain words offer nothing: ${words.slice(0, 12)}`,
                Sprouting.offered(words, "from:TERN02", sessionNamed), null);
        Expect.same("the foot says a page offered it, from the session by its quest",
            Sprouting.offeredSaid({ words: "tidy", from: { address: "TERN02",
                name: "Tidy the Rows" } }, said),
            "a page offered this work, saying it's from Tidy the Rows; read it to its end, then Return starts it");
        Expect.same("and only that a page offered it, from no session here",
            Sprouting.offeredSaid({ words: "tidy", from: null }, said),
            "a page offered this work; read it to its end, then Return starts it");

        // Why it asks nothing, said where Joel pressed.
        Expect.same("nothing said asks for the work",
            Sprouting.refused(" ", "", said), "say what the work is");
        Expect.same("a folder not whole says so",
            Sprouting.refused("tidy it", "src/core", said), "that folder isn't a whole path");
        Expect.same("a sprout it can ask is no refusal",
            Sprouting.refused("tidy it", "/home/j/src/core", said), null);

        // How it ended, as the daemon answers (modules/agents/src/sprout.rs).
        Expect.same("started, on the machine the node chose, held by the id it gave",
            Sprouting.ended({ id: "ask-3", sprouted: { machine: "kestrel", held: "h1",
                folder: "/home/j/src/core", attach: ["mycelium", "join", "h1"] } }, said),
            { started: true, machine: "kestrel", held: "h1" });
        Expect.same("started with no holder id says no id",
            Sprouting.ended({ sprouted: { machine: "kestrel" } }, said),
            { started: true, machine: "kestrel", held: "" });
        Expect.same("refused, in the node's or the daemon's words",
            Sprouting.ended({ id: "ask-3", error: { code: "unsprouted",
                message: "only the launcher starts new work here" } }, said),
            { started: false, why: "only the launcher starts new work here", refusal: "" });
        Expect.same("refused with the node's code for it, which goes with its words",
            Sprouting.ended({ error: { code: "unsprouted", message: "no folder",
                refusal: "needs_folder" } }, said),
            { started: false, why: "no folder", refusal: "needs_folder" });
        Expect.same("a node the daemon heard fall silent is unanswered, in its words",
            Sprouting.ended({ error: { code: "unanswered", message: "it may still start it" } },
                said), { started: false, unanswered: true, why: "it may still start it" });
        Expect.same("no daemon to ask is no answer",
            Sprouting.ended(null, said),
            { started: false, unanswered: true, why: "its node didn't answer" });
        Expect.same("an answer that isn't a sprout's is unread",
            Sprouting.ended({ id: "ask-3", sprouted: {} }, said),
            { started: false, why: "its node's answer couldn't be read" });
        Expect.same("an error with no words is unread",
            Sprouting.ended({ error: { code: "unsprouted" } }, said),
            { started: false, why: "its node's answer couldn't be read" });

        // Which running session is the one it started, to open for him (Joel, 18 November:
        // "start new work does not seem to work from the launcher"): here, by its holder's
        // id; on the machine the node named, by the words that attach it, which end in that
        // id, as PiSource.js keeps a session elsewhere (here: false, held: null).
        const here = { address: "ONE", here: true, held: "h1", attach: null };
        const there = { address: "TWO", here: false, machine: "kestrel", held: null,
            attach: ["mycelium", "--realm", "work", "attach", "h2"] };
        const other = { address: "THREE", here: true, held: "h3",
            attach: ["mycelium", "attach", "h3"] };
        Expect.same("one held here is found by its holder's id",
            Sprouting.sproutedOf([other, here], "h1", "heron")?.address, "ONE");
        Expect.same("one on another machine is found by its attach words",
            Sprouting.sproutedOf([other, there], "h2", "kestrel")?.address, "TWO");
        Expect.same("none is found before it's up",
            Sprouting.sproutedOf([other], "h1", "heron"), null);
        Expect.same("no id finds none, never the first held",
            Sprouting.sproutedOf([{ address: "FOUR", here: true, held: "", attach: [] }], "",
                "heron"), null);
        Expect.same("the same id on another machine than the node named finds none",
            Sprouting.sproutedOf([there], "h2", "falcon"), null);
        Expect.same("a session here whose attach words name the id isn't it",
            Sprouting.sproutedOf([{ address: "FIVE", here: true, held: "h9",
                attach: ["mycelium", "attach", "h2"] }], "h2", "kestrel"), null);
        Expect.same("attach words naming the id but not at their end aren't it",
            Sprouting.sproutedOf([{ address: "SIX", here: false, machine: "kestrel", held: null,
                attach: ["mycelium", "attach", "h2", "--then", "x"] }], "h2", "kestrel"), null);

        // Whether Joel still waits on the act in the launcher, so it may close for its
        // window and take the keys: open, on the act itself or held in its ask, where
        // Return leaves him with the field emptied for another try, and no key taken
        // since the answer (its keysTaken then), so a new try he's typing keeps its keys.
        const act = "verb:agents:start-new-work:link";
        const at = (open, asking, selected, keysTaken) => ({ open, asking, selected,
            keysTaken, lettersAt: 0 });
        Expect.same("held in the act's ask, it waits",
            Sprouting.waitsOn(at(true, { key: act }, null, 5), act, 5), true);
        Expect.same("on the act's row, it waits",
            Sprouting.waitsOn(at(true, null, { key: act }, 5), act, 5), true);
        Expect.same("a key taken since, in its ask, it doesn't",
            Sprouting.waitsOn(at(true, { key: act }, null, 9), act, 5), false);
        // An arrow sets lettersAt to 0 again, so an answer heard after one, at 0, would
        // wait again at any later arrow; keysTaken only goes up.
        Expect.same("an arrow since, on the act's row, it doesn't",
            Sprouting.waitsOn(at(true, null, { key: act }, 6), act, 0), false);
        Expect.same("one with no launcher at its answer never does",
            Sprouting.waitsOn(at(true, null, { key: act }, 0), act, -1), false);
        Expect.same("on another row, it doesn't",
            Sprouting.waitsOn(at(true, null, { key: "other" }, 5), act, 5), false);
        Expect.same("closed, it doesn't",
            Sprouting.waitsOn(at(false, { key: act }, null, 5), act, 5), false);
        Expect.same("no launcher, it doesn't", Sprouting.waitsOn(null, act, 5), false);

        // How the waiting stand at `now`: each up opens however long it took, as Joel's
        // start at 14:30 on 25 November came up five and a half minutes on; one past the
        // minute and not yet said to be late is late, once; one past the watch is gone.
        {
        const minutes = 60000;
        const times = { patience: minutes, watch: 15 * minutes };
        const now = 20 * minutes;
        const openings = [
            { key: "slow", held: "h1", since: now - 5.5 * minutes, late: true },
            { key: "quick", held: "h2", since: now - 5000 },
            { key: "waits", held: "h3", since: now - 10000 },
            { key: "late", held: "h4", since: now - 2 * minutes },
            { key: "said", held: "h5", since: now - 3 * minutes, late: true },
            { key: "gone", held: "h6", since: now - 15 * minutes, late: true },
            { key: "unmarked", held: "h7", since: now - 2 * minutes },
            { key: "watched", held: "h8", since: now - 16 * minutes, late: true }
        ];
        const upAre = new Set(["h1", "h2", "h7", "h8"]);
        const found = one => upAre.has(one.held) ? { address: one.held } : null;
        const stood = Sprouting.settled(openings, found, now, times);
        Expect.same("each up is found, with its session, the slow one after its minute too",
            stood.up.map(one => [one.key, one.session.address]),
            [["slow", "h1"], ["quick", "h2"], ["unmarked", "h7"], ["watched", "h8"]]);
        Expect.same("each up keeps its late mark, which picks the words said of it",
            stood.up.map(one => Openings.upWord(one)), ["upLater", "up", "up", "upLater"]);
        Expect.same("an up keeps the words said of it late, which it takes back",
            Sprouting.settled([{ key: "k", held: "h9", machine: "m", since: 0, late: true,
                told: "late on m" }], () => ({ address: "h9" }), now, times).up[0].told,
            "late on m");
        Expect.same("one past the minute is late", stood.late.map(one => one.key), ["late"]);
        Expect.same("one watched a quarter hour is gone", stood.gone.map(one => one.key),
            ["gone"]);
        Expect.same("one within the minute, or said late, waits on, and one late now too",
            stood.waiting.map(one => [one.key, one.late === true]),
            [["waits", false], ["late", true], ["said", true]]);
        Expect.same("one said late is said late once", Sprouting.settled(stood.waiting,
            () => null, now + 5000, times).late.length, 0);
        Expect.same("at the minute's own end, it's late", Sprouting.settled(
            [{ key: "x", since: now - minutes }], () => null, now, times).late.length, 1);
        Expect.same("one said late, a minute short of the quarter hour, still waits",
            Sprouting.settled([{ key: "x", since: now - 14 * minutes, late: true }],
                () => null, now, times).waiting.length, 1);
        {
            const toldOne = [{ key: "t", held: "h0", since: now - 3 * minutes, late: true,
                told: "late on m" }];
            Expect.same("one waiting keeps the words said of it late", Sprouting.settled(
                toldOne, () => null, now, times).waiting[0].told, "late on m");
            Expect.same("one gone keeps them too, so it takes them back", Sprouting.settled(
                toldOne, () => null, now + 13 * minutes, times).gone[0].told, "late on m");
        }
        }

        // What the act that ran says.
        Expect.same("started names the machine",
            Sprouting.told({ started: true, machine: "kestrel", held: "h1" }, said),
            "started on kestrel");
        Expect.same("started with no holder's id promises no window",
            Sprouting.told({ started: true, machine: "kestrel", held: "" }, said),
            "started on kestrel; in Agents once up");
        Expect.same("refused says why",
            Sprouting.told({ started: false, why: "no machine can take it" }, said),
            "no machine can take it");

        // A refusal one of the launcher's acts gets past ends with that act, the
        // command line's own cut, in the node's or the daemon's words as each says it:
        // mycelium 8f1b0e66's sprout.rs folder and node placement, and Rust's words
        // for a missing socket and a refused one in the agents daemon's sprout.rs.
        const refusal = why => Sprouting.told({ started: false, why }, said);
        Expect.same("one needing a repository says the launcher's act, not the command line's",
            refusal("it needs a repository to work in, and /home/j isn't one: say which, as "
                + "`mycelium sprout --in <folder> ...`"),
            "it needs a repository to work in, and /home/j isn't one; start it In a repository");
        Expect.same("and as a node that names its places says it",
            refusal("it needs a repository to work in, and /home/j isn't one: say which, as "
                + "`mycelium sprout --in <folder or place> ...`; `mycelium places` lists the "
                + "places"),
            "it needs a repository to work in, and /home/j isn't one; start it In a repository");
        Expect.same("one begun at home names the act, not the folder to name",
            refusal("a session begun in a home folder has none of its repository's skills: "
                + "name the repository's folder"),
            "a session begun in a home folder has none of its repository's skills; "
                + "start it In a repository");
        Expect.same("a folder not here is started In another",
            refusal("/home/j/src/gone isn't a folder here"),
            "/home/j/src/gone isn't a folder here; start it In a repository");
        Expect.same("one placed nowhere says to say it again once a machine can take it",
            refusal("nowhere: skiff lacks system macos; tern is out of reach"),
            "nowhere: skiff lacks system macos; tern is out of reach; "
                + "once one can take it, say it again");
        for (const never of ["nowhere: skiff lacks system macos",
            "nowhere: skiff lacks system macos, is out of reach; tern isn't kestrel",
            "nowhere: skiff is out of reach, lacks system macos",
            "nowhere: the realm knows no machine"]) {
            Expect.same(`one no machine will ever take is told in its own words: ${never}`,
                refusal(never), never);
        }
        Expect.same("one placed nowhere now, a machine out of reach would do, says it too",
            refusal("nowhere now, though a machine out of reach would do: tern is out of reach"),
            "nowhere now, though a machine out of reach would do: tern is out of reach; "
                + "once one can take it, say it again");
        Expect.same("one a machine lacking nothing may yet take says to say it again",
            refusal("nowhere: skiff lacks tool gh; tern holds the 8 sessions it holds at most"),
            "nowhere: skiff lacks tool gh; tern holds the 8 sessions it holds at most; "
                + "once one can take it, say it again");
        Expect.same("one whose node's socket is missing says to say it again once it runs",
            refusal("the node couldn't be reached: No such file or directory (os error 2)"),
            "the node couldn't be reached: No such file or directory (os error 2); "
                + "once the node runs, say it again");
        Expect.same("and one whose node refused",
            refusal("the node couldn't be reached: Connection refused (os error 111)"),
            "the node couldn't be reached: Connection refused (os error 111); "
                + "once the node runs, say it again");
        Expect.same("one the node's socket won't let in isn't a node that's down",
            refusal("the node couldn't be reached: Permission denied (os error 13)"),
            "the node couldn't be reached: Permission denied (os error 13)");
        Expect.same("one whose command line's act isn't where it's cut keeps only its words",
            refusal("it needs a repository to work in, and /home/j isn't one: say which, as "
                + "`mycelium sprout --in <folder> ...`."),
            "it needs a repository to work in, and /home/j isn't one: say which, as "
                + "`mycelium sprout --in <folder> ...`.");
        Expect.same("and one begun at home whose folder to name runs on",
            refusal("a session begun in a home folder has none of its repository's skills: "
                + "name the repository's folder, or a link"),
            "a session begun in a home folder has none of its repository's skills: "
                + "name the repository's folder, or a link");
        Expect.same("words quoting a repository's need further in aren't that refusal",
            refusal("this asks as a fence, so it runs only there: it needs a repository to "
                + "work in"),
            "this asks as a fence, so it runs only there: it needs a repository to work in");

        // A start refused for want of a repository asks Joel where it starts, his
        // words carried to the folder he picks, rather than only saying so on the row
        // he gave them from (Joel, 24 November: "the UI was crap").
        const home = "it needs a repository to work in, and /home/j isn't one: say which, as "
            + "`mycelium sprout --in <folder> ...`";
        const unskilled = "a session begun in a home folder has none of its repository's "
            + "skills: name the repository's folder";
        for (const why of [home, unskilled, "/home/j/src/gone isn't a folder here"]) {
            Expect.same(`one wanting a repository asks where: ${why.slice(0, 24)}`,
                Sprouting.asksWhere({ started: false, why }), true);
        }
        for (const how of [{ started: true, machine: "kestrel", held: "h1" },
            { started: false, unanswered: true, why: home },
            { started: false, why: "nowhere: tern is out of reach" },
            { started: false, why: "this asks as a fence, so it runs only there: it needs a "
                + "repository to work in" },
            { started: false, why: "" },
            { started: false, why: "no folder", refusal: "no_such_place" },
            { started: false, unanswered: true, why: "no folder", refusal: "needs_folder" }]) {
            Expect.same(`any other end asks nowhere: ${JSON.stringify(how).slice(0, 48)}`,
                Sprouting.asksWhere(how), false);
        }
        Expect.same("asked where, it says the refusal and that his words go with the folder",
            Sprouting.toldWhere({ started: false, why: home }, said),
            "it needs a repository to work in, and /home/j isn't one; "
                + "pick where it starts below; your words go with it");
        Expect.same("and one begun at home",
            Sprouting.toldWhere({ started: false, why: unskilled }, said),
            "a session begun in a home folder has none of its repository's skills; "
                + "pick where it starts below; your words go with it");
        // The node codes it needs_folder, whatever its words (mycelium's Refused).
        const coded = { started: false, why: "say where it starts", refusal: "needs_folder" };
        Expect.same("one the node codes as needing a folder asks where in any words",
            [Sprouting.asksWhere(coded), Sprouting.toldWhere(coded, said)],
            [true, "say where it starts; pick where it starts below; your words go with it"]);
        const stillIn = { open: true, asking: { key: "link" }, lettersAt: 7 };
        Expect.same("he's stepped back while still in the act's emptied field",
            Sprouting.stepsBack(stillIn, "link", 7, 2), true);
        const away = [["the launcher shut", Object.assign({}, stillIn, { open: false }), 2],
            ["at another act", Object.assign({}, stillIn, { asking: { key: "in" } }), 2],
            ["on its row, out of the field", Object.assign({}, stillIn, { asking: false }), 2],
            ["a key pressed since", Object.assign({}, stillIn, { lettersAt: 9 }), 2],
            ["no folder to pick", stillIn, 0], ["no launcher", null, 2]];
        for (const [why, launcher, folders] of away) {
            Expect.same(`nor stepped anywhere with ${why}`,
                Sprouting.stepsBack(launcher, "link", 7, folders), false);
        }
        Expect.same("with those words missing, it's told as any refusal is",
            Sprouting.toldWhere({ started: false, why: home },
                Object.assign({}, said, { askedWhere: "" })),
            "it needs a repository to work in, and /home/j isn't one; start it In a repository");
        Expect.same("one unanswered keeps its own words, which say what to do",
            Sprouting.told({ started: false, unanswered: true,
                why: "the node couldn't be reached in time" }, said),
            "the node couldn't be reached in time");
        Expect.same("even words a refusal's act would end, since it may yet start",
            Sprouting.told({ started: false, unanswered: true,
                why: "nowhere: kestrel is full" }, said),
            "nowhere: kestrel is full");
        Expect.same("with the act's words missing, it's told as it was said",
            Sprouting.told({ started: false, why: "nowhere: kestrel is full" }, {}),
            "nowhere: kestrel is full");
        Expect.same("words that only hold a known refusal's aren't it",
            refusal("it said nowhere: and stopped"), "it said nowhere: and stopped");
        const roomless = { started: false,
            why: "nowhere: kestrel holds the 8 sessions it holds at most" };
        Expect.same("the act is told Joel, and the journal keeps the node's words",
            Sprouting.reach(roomless, said), {
                journal: "new work wasn't started: nowhere: kestrel holds the 8 sessions it "
                    + "holds at most",
                told: "nowhere: kestrel holds the 8 sessions it holds at most; "
                    + "once one can take it, say it again"
            });
        // Each act is said in words Joel reads: a key missing from Start New Work's
        // words would leave its refusals with no act, as one unknown is.
        const written = writtenSaid();
        Expect.same("each refusal's act is in Start New Work's own words",
            Sprouting.refusalActs().filter(key => String(written[key] ?? "").trim() === ""), []);
        Expect.same("and so is what asking where says",
            String(written.askedWhere ?? "").trim() !== "", true);
        // What an opening says (Agents.qml's said): each written, each up one naming its
        // session, and none filling more than it gives.
        const opened = ["opening", "up", "upLater", "unopened", "lost"];
        Expect.same("each word an opening says is written",
            opened.filter(key => String(written[key] ?? "").trim() === ""), []);
        Expect.same("each said once it's up names its session",
            ["up", "upLater"].filter(key => !String(written[key]).includes("{title}")), []);
        const full = { machine: "kestrel", title: "Olga's access", minutes: "15" };
        // Agents.qml's said() gives every word a title, empty until the session is up, so
        // only the up words may name one, or a word said before would show it blank.
        Expect.same("only an up word names the title it's given", opened.filter(key =>
            String(written[key]).includes("{title}")), ["up", "upLater"]);
        Expect.same("each is filled whole", opened.filter(key =>
            Openings.filled(written, key, full).includes("{")), []);

        // How an opening's words are filled and where they're said.
        Expect.same("a title is only text, its $ patterns kept", Openings.filled(
            { up: "{title} is up on {machine}" }, "up", { machine: "k", title: "a $& b $$" }),
            "a $& b $$ is up on k");
        Expect.same("a name not given is filled with nothing",
            Openings.filled({ lost: "{minutes} min" }, "lost", {}), " min");
        Expect.same("an act unpressed since says", Openings.saysOnAct({ a: 100 }, "a", 200),
            true);
        Expect.same("an act never pressed says", Openings.saysOnAct({}, "a", 200), true);
        {
            const opening = { key: "a", since: 200 };
            Expect.same("an up start Joel waits on, its act unpressed since, takes the keys",
                Openings.takesKeys(true, { a: 100 }, opening), true);
            Expect.same("its act offered fresh words since, it doesn't, though he waits",
                Openings.takesKeys(true, { a: 300 }, opening), false);
            Expect.same("not waited on, it doesn't", Openings.takesKeys(false, {}, opening),
                false);
            // Made from its Return, at 200 with 7 keys taken, though the node answered
            // later: a key or an offer while it was asked counts against it.
            const made = Openings.opening({ held: "h1", machine: "kestrel" }, "a",
                { words: "w", letters: 0, keys: 7, at: 200 });
            Expect.same("an opening counts from its Return", made,
                { key: "a", held: "h1", machine: "kestrel", since: 200, keys: 7 });
            Expect.same("an offer while it was asked keeps the launcher open",
                Openings.takesKeys(true, { a: 250 }, made), false);
            Expect.same("once the launcher shut, none takes the keys again",
                Openings.shutOn([made, { key: "b", keys: 3, late: true }])
                    .map(one => [one.key, one.keys, one.late === true]),
                [["a", -1, false], ["b", -1, true]]);
        }
        Expect.same("an act pressed since, or given fresh words, doesn't",
            Openings.saysOnAct({ a: 300 }, "a", 200), false);
        Expect.same("an act pressed as it was answered still says",
            Openings.saysOnAct({ a: 200 }, "a", 200), true);
        Expect.same("a press stamps only its own act, at its time",
            Openings.pressedAt({ a: 100, b: 5 }, "a", 300), { a: 300, b: 5 });
        Expect.same("a later press of its act stops a start saying over it",
            Openings.saysOnAct(Openings.pressedAt({}, "a", 300), "a", 200), false);
        const waitingNow = [{ held: "h1", machine: "k" }, { held: "h2", machine: "k" },
            { held: "h1", machine: "m" }];
        Expect.same("each late opening carries what was said of it, and no other",
            Openings.toldOf(waitingNow, [{ held: "h2", machine: "k" },
                { held: "h1", machine: "m" }], ["late h2", "late h1 on m"]),
            [{ held: "h1", machine: "k" }, { held: "h2", machine: "k", told: "late h2" },
                { held: "h1", machine: "m", told: "late h1 on m" }]);
        const tells = { told: ["late on k", "lost on m", "late on k"], keys: 3, at: 9,
            shown: [["n", "late on k"]] };
        const trimmed = Openings.footedWithout(tells, "late on k");
        Expect.same("an opening takes back only its own tell, once",
            [trimmed.told, trimmed.keys, trimmed.at],
            [["lost on m", "late on k"], 3, 9]);
        Expect.same("what's left keeps the foot it was shown in, which shut clears by",
            trimmed.shown === tells.shown, true);
        Expect.same("a tell it never had leaves the foot as it was",
            Openings.footedWithout(tells, "up on q"), tells);
        Expect.same("none is left once its own was the only tell",
            Openings.footedWithout({ told: ["late on k"], keys: 1, at: 2 }, "late on k"), null);
        Expect.same("no foot stays none", Openings.footedWithout(null, "late on k"), null);
        const shownFoot = [["n", "late on k"]];
        const lone = { told: ["late on k"], keys: 1, at: 2, shown: shownFoot };
        const back = Openings.takenBack(lone, { told: "late on k" });
        Expect.same("taking back the foot's last tell leaves none, and clears its foot",
            [back.footed, back.cleared === shownFoot], [null, true]);
        const someLeft = Openings.takenBack(tells, { told: "late on k" });
        Expect.same("taking back one of several clears no foot",
            [someLeft.footed.told, someLeft.cleared], [["lost on m", "late on k"], null]);
        Expect.same("one never said late takes back nothing",
            Openings.takenBack(lone, {}), { footed: lone, cleared: null });
        Expect.same("no foot has none to clear",
            Openings.takenBack(null, { told: "late on k" }), { footed: null, cleared: null });

        // Where each end is said beyond its own row, which Joel may have left.
        const refusedEnd = { started: false, why: "/home/j isn't a repository's folder" };
        const startedEnd = { started: true, machine: "kestrel", held: "h1" };
        Expect.same("one refused is told why, and journalled",
            Sprouting.reach(refusedEnd, said), {
                journal: "new work wasn't started: /home/j isn't a repository's folder",
                told: "/home/j isn't a repository's folder"
            });
        Expect.same("one unanswered is told, and journalled as one that may yet start",
            Sprouting.reach(Sprouting.ended(null, said), said), {
                journal: "new work's start went unanswered; it may still start",
                told: said.unanswered
            });
        Expect.same("one started with no holder's id opens no window, so it's told",
            Sprouting.reach({ started: true, machine: "kestrel", held: "" }, said), {
                journal: "new work started on kestrel",
                told: "started on kestrel; in Agents once up"
            });
        Expect.same("one held is journalled, and says itself as it comes up",
            Sprouting.reach(startedEnd, said),
            { journal: "new work started on kestrel", told: null });

        // What the foot told is noticed as the launcher shuts unless it was read: a key
        // pressed since and ten seconds gone, since the keys that shut it are a leaving.
        const footed = { keys: 4, at: 1000 };
        Expect.same("shut with no key since, it's noticed",
            Sprouting.noticedAtShut(footed, 4, 60000), true);
        Expect.same("shut by keys soon after, it's noticed",
            Sprouting.noticedAtShut(footed, 7, 4000), true);
        Expect.same("shut long after a key, it was read",
            Sprouting.noticedAtShut(footed, 5, 20000), false);
        Expect.same("nothing footed, nothing noticed",
            Sprouting.noticedAtShut(null, 4, 4000), false);
        Expect.same("a notice's body is each end, a line each, as plain text",
            Sprouting.noticeBody(["<a href=\"x\">look</a> & see", "busy"]),
            "&lt;a href=\"x\"&gt;look&lt;/a&gt; &amp; see\nbusy");

        // The starts the agents daemon keeps ended and untold: one this shell never heard
        // the end of is told once it has stood kept a moment, first seen at `seen`, and each
        // but one it's still asking or waiting out is said told.
        const now = 1763500000000;
        const kept = [
            { token: "a-1", ended: { error: { code: "unanswered", message: "cut off" } } },
            { token: "a-2", ended: { sprouted: { machine: "kestrel", held: "h2" } } },
            { token: "a-3", ended: { error: { code: "unsprouted", message: "no" } } },
            { token: "a-4", ended: { error: { code: "unsprouted", message: "no" } } }
        ];
        const known = { "a-2": "ended", "a-3": "asking", "a-4": "told" };
        const settled = { "a-1": now - Sprouting.SETTLE_MS };
        const untold = Sprouting.untold(kept, known, settled, now);
        Expect.same("one never heard is told", untold.tell.map(one => one.token), ["a-1"]);
        Expect.same("each heard or told is said told, and one still asked isn't",
            untold.told, ["a-1", "a-2", "a-4"]);
        Expect.same("none waits once each never heard has settled", untold.waiting, []);
        // One still asked whose answer was lost, as the daemon went, is answered by the
        // record the daemon kept, so the shell that asked tells it, and only once it's kept.
        Expect.same("one still asked is answered by its record",
            untold.answered.map(one => one.token), ["a-3"]);
        // A shell that never asked tells only those the daemon still keeps as it asks,
        // since its asker may have told it since its own copy was served: the daemon
        // answers sprout-kept from the file it keeps, which it reads before it listens.
        Expect.same("one told since isn't told again",
            Sprouting.stillKept(untold.tell, { kept: ["a-2"] }), []);
        Expect.same("one still kept is told", Sprouting.stillKept(untold.tell,
            { kept: ["a-1"] }).map(one => one.token), ["a-1"]);
        Expect.same("with no answer, none is told yet",
            Sprouting.stillKept(untold.tell, null), []);
        Expect.same("nor with a refusal, as from a daemon before sprout-kept",
            Sprouting.stillKept(untold.tell, { error: { code: "unknown-ask" } }), []);
        // Its asker, still up beside this shell while a switch hands over, says it told
        // first, so one first seen just now waits rather than being told twice.
        const fresh = Sprouting.untold(kept, known, {}, now);
        Expect.same("one just seen isn't told yet", fresh.tell, []);
        Expect.same("nor said told, as it isn't", fresh.told, ["a-2", "a-4"]);
        Expect.same("it waits", fresh.waiting, ["a-1"]);
        const early = Sprouting.untold(kept, known, { "a-1": now - Sprouting.SETTLE_MS + 1 },
            now);
        Expect.same("one seen less than the moment ago still waits", early.waiting, ["a-1"]);
        // Its asker may hear it later than this shell does, as each comes back to a daemon
        // started again in its own time, but tells it within its patience of the asking,
        // so one asked more recently than that waits, however long it's been seen.
        const recent = [{ token: "b-1", at: now - Sprouting.ASKER_ANSWERS_MS + 1, ended: {} }];
        const lately = Sprouting.untold(recent, {}, { "b-1": now - Sprouting.SETTLE_MS }, now);
        Expect.same("one asked within its asker's patience waits", lately.waiting, ["b-1"]);
        Expect.same("and isn't told", lately.tell, []);
        const asked = [{ token: "b-1", at: now - Sprouting.ASKER_ANSWERS_MS, ended: {} }];
        Expect.same("one asked longer ago than that is told", Sprouting.untold(asked, {},
            { "b-1": now - Sprouting.SETTLE_MS }, now).tell.map(one => one.token), ["b-1"]);
        Expect.same("the asker's patience outlasts the daemon's",
            Sprouting.PATIENCE_MS > 50000 && Sprouting.ASKER_ANSWERS_MS > Sprouting.PATIENCE_MS,
            true);
        // A token the daemon took is letters, digits and dashes, as an object's own
        // inherited names are too, so only a token this shell holds counts as known.
        const named = Sprouting.untold([{ token: "constructor", ended: {} }], {},
            { constructor: now - Sprouting.SETTLE_MS }, now);
        Expect.same("an inherited name isn't known", named.tell.map(one => one.token),
            ["constructor"]);
        Expect.same("none kept, none told", Sprouting.untold([], {}, {}, now),
            { tell: [], told: [], waiting: [], answered: [] });
        const token = Sprouting.token(1763500000000, 0.5);
        Expect.same("a start's token is letters, digits and a dash, as the daemon keeps",
            /^[a-z0-9]+-[a-z0-9]+$/u.test(token), true);
        Expect.same("two asked in one moment are told apart",
            token === Sprouting.token(1763500000000, 0.25), false);
    }
}
