// Start New Work, Agents' act that begins new work from a link or words, run
// in Qt's own JavaScript engine, the one the shell runs it in. The
// `start-new-work-asks-only-a-sprout` check copies
// modules/quickshell/launcher/Sprouting.js and tests/lib/Expect.js beside
// this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
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
        unplaced: "that folder isn't a whole path"
    })

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
            { started: false, why: "only the launcher starts new work here" });
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
        // Return leaves him with the field emptied for another try, and no letter typed
        // since the answer (its lettersAt then), so a new try he's typing keeps its keys.
        const act = "verb:agents:start-new-work:link";
        const at = (open, asking, selected, lettersAt) => ({ open, asking, selected,
            lettersAt });
        Expect.same("held in the act's ask, it waits",
            Sprouting.waitsOn(at(true, { key: act }, null, 5), act, 5), true);
        Expect.same("on the act's row, it waits",
            Sprouting.waitsOn(at(true, null, { key: act }, 5), act, 5), true);
        Expect.same("a letter typed since, in its ask, it doesn't",
            Sprouting.waitsOn(at(true, { key: act }, null, 9), act, 5), false);
        Expect.same("on another row, it doesn't",
            Sprouting.waitsOn(at(true, null, { key: "other" }, 5), act, 5), false);
        Expect.same("closed, it doesn't",
            Sprouting.waitsOn(at(false, { key: act }, null, 5), act, 5), false);
        Expect.same("no launcher, it doesn't", Sprouting.waitsOn(null, act, 5), false);

        // How the waiting stand at `now`: those past the minute are late even when up by
        // then, so none opens past it; those up within it open; the rest wait on.
        const openings = [{ key: "a", held: "h1", since: 1000 }, { key: "b", held: "h2",
            since: 50000 }, { key: "c", held: "h3", since: 55000 }];
        const upAre = new Set(["h1", "h2"]);
        const found = one => upAre.has(one.held) ? { address: one.held } : null;
        const stood = Sprouting.settled(openings, found, 61000, 60000);
        Expect.same("one past the minute is late, though up",
            stood.late.map(one => one.key), ["a"]);
        Expect.same("one up within it opens, with its session",
            stood.up.map(one => [one.key, one.session.address]), [["b", "h2"]]);
        Expect.same("one not up within it waits on", stood.waiting.map(one => one.key), ["c"]);
        Expect.same("at the minute's own end, it's late",
            Sprouting.settled([openings[0]], () => null, 61000, 60000).late.length, 1);

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
        // since its asker may have told it since its own copy was served.
        Expect.same("one told since isn't told again",
            Sprouting.stillKept(untold.tell, [{ key: "start:agents:a-2" }]), []);
        Expect.same("one still kept is told", Sprouting.stillKept(untold.tell,
            [{ key: "start:agents:a-1" }]).map(one => one.token), ["a-1"]);
        Expect.same("with no answer, none is told yet",
            Sprouting.stillKept(untold.tell, null), []);
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
