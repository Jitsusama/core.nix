// How the shell takes a place's rows as its daemon serves them ready,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-launcher-words-each-ready-row-it-is-served` check copies
// launcher/ReadyRows.js, the When.js it reads from beside the launcher's
// folder, launcher/Ago.js, launcher/Interfaces.js and tests/lib/Expect.js
// beside this file and runs `qml -platform offscreen ready-rows.qml` over
// records made up here in the crate's wrapped form.
import QtQuick
import "launcher/ReadyRows.js" as Ready
import "Expect.js" as Expect

QtObject {
    // The launcher's words for times, as words.json's field.times gives
    // them, with the calendar's names and a zone said by its name.
    readonly property var said: ({
            now: "just now",
            minutes: "{minutes} min ago",
            yesterday: "yesterday",
            tomorrow: "tomorrow",
            today: "today",
            happening: "now",
            soon: "in {minutes} min",
            at: "at {time}",
            opened: "opened {when}",
            date: (date, format) => Qt.formatDate(date, format),
            zone: zone => `${zone}'s time`
        })

    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            placing();
            wording();
            sides();
            drawers();
            ticking();
            insides();
            acting();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    function record(key, order) {
        return { key: key, kind: "row", row: { thing: `quest:${key}`, order: order } };
    }

    function placing() {
        const held = {
            a: record("a", 1024),
            b: record("b", 2048),
            c: record("c", 3072),
            "summary:quests": { key: "summary:quests", kind: "summary", summary: { said: "3" } }
        };
        Expect.same("a record wrapped as a row gives its row", Ready.rowOf(held.a), held.a.row);
        Expect.same("a summary beside the rows gives none", Ready.rowOf(held["summary:quests"]),
            null);
        let keys = Ready.placed([], held, ["c", "summary:quests", "a", "b"], []);
        Expect.same("a whole answer is placed by order, and only its rows", keys,
            ["a", "b", "c"]);
        held.c = record("c", 1536);
        const before = keys;
        keys = Ready.placed(keys, held, ["c"], []);
        Expect.same("a row moved midway between two goes between them", keys, ["a", "c", "b"]);
        Expect.same("and the list before is left as it was", before, ["a", "b", "c"]);
        held.d = record("d", 0);
        delete held.b;
        keys = Ready.placed(keys, held, ["d"], ["b"]);
        Expect.same("a new row takes its place and a row gone is let go", keys,
            ["d", "a", "c"]);
        held.e = record("e", 1024);
        keys = Ready.placed(keys, held, ["e"], []);
        Expect.same("a row ordered as another goes after it", keys, ["d", "a", "e", "c"]);
    }

    function wording() {
        const now = new Date(2026, 10, 11, 14, 0).getTime();
        const at = (day, hour, minute) => new Date(2026, 10, day, hour, minute).getTime();
        const minutes = count => now - count * 60_000;
        const say = time => Ready.timeSaid(time, now, said);
        Expect.same("came within a minute", say({ at: now - 30_000, as: "came" }), "just now");
        Expect.same("came within the hour", say({ at: minutes(12), as: "came" }), "12 min ago");
        Expect.same("came earlier today", say({ at: at(11, 9, 5), as: "came" }), "09:05");
        Expect.same("came yesterday", say({ at: at(10, 9, 5), as: "came" }), "yesterday");
        Expect.same("opened", say({ at: minutes(12), as: "opened" }), "opened 12 min ago");
        Expect.same("a meeting on", say({ at: minutes(10), until: minutes(-20), as: "meeting" }),
            "now");
        Expect.same("a meeting within the hour", say({ at: minutes(-20), as: "meeting" }),
            "in 20 min");
        Expect.same("a meeting later today", say({ at: at(11, 16, 0), as: "meeting" }),
            "at 16:00");
        Expect.same("a meeting tomorrow", say({ at: at(12, 9, 0), as: "meeting" }),
            "tomorrow at 09:00");
        Expect.same("an all-day meeting today",
            say({ at: at(11, 0, 0), allDay: true, as: "meeting" }), "today");
        Expect.same("a meeting over today says when it was held",
            say({ at: at(11, 10, 0), until: at(11, 11, 0), as: "meeting" }), "10:00");
        Expect.same("a meeting over before today says its day and time",
            say({ at: at(10, 10, 0), until: at(10, 11, 0), as: "meeting" }), "yesterday 10:00");
        Expect.same("a moment today", say({ at: at(11, 9, 5), as: "moment" }), "09:05");
        Expect.same("a moment on another day", say({ at: at(10, 9, 5), as: "moment" }),
            "yesterday 09:05");
        Expect.same("dated this year", say({ at: at(11, 9, 5), as: "dated" }),
            "Wednesday 11 November, 09:05");
        Expect.same("dated another year",
            say({ at: new Date(2025, 0, 5, 9, 5).getTime(), as: "dated" }),
            "Sunday 5 January 2025, 09:05");
        Expect.same("ago", say({ at: minutes(180), as: "ago" }), "3 h");
        Expect.same("lasted", say({ at: now - (3 * 24 + 2) * 3_600_000, as: "lasted" }),
            "3 d 2 h");
        Expect.same("a day this year", say({ at: new Date(2026, 9, 22).getTime(), as: "day" }),
            "22 Oct");
        Expect.same("a day another year",
            say({ at: new Date(2025, 0, 5).getTime(), as: "day" }), "5 Jan 2025");
        Expect.same("a time somewhere else", say({ zone: "Europe/Oslo", as: "zone" }),
            "Europe/Oslo's time");
        Expect.same("a form it doesn't know says nothing", say({ at: now, as: "someday" }), "");
    }

    function sides() {
        const now = new Date(2026, 10, 11, 14, 0).getTime();
        const morning = new Date(2026, 10, 11, 9, 5).getTime();
        const side = [{ said: "3 sessions" }, { at: now - 3 * 3_600_000, as: "ago" }, { said: "" }];
        Expect.same("a side's items are joined by a dot, those saying nothing left out",
            Ready.sideSaid(side, now, said), "3 sessions \u00b7 3 h");
        Expect.same("no side says nothing", Ready.sideSaid(undefined, now, said), "");
        Expect.same("a time worded into its words",
            Ready.itemSaid({ at: morning, as: "moment", said: "since {time}" }, now, said),
            "since 09:05");
        Expect.same("a time whose words hold no place for it is said alone",
            Ready.itemSaid({ at: morning, as: "moment", said: "since" }, now, said), "09:05");
        Expect.same("a note of words is said as it is",
            Ready.noteSaid("a password", now, said), "a password");
        Expect.same("a note that's a time is worded",
            Ready.noteSaid({ at: morning, as: "moment", said: "since {time}" }, now, said),
            "since 09:05");
    }

    function drawers() {
        const now = new Date(2026, 10, 11, 14, 0).getTime();
        Expect.same("no drawer served is none", Ready.drawerOf(undefined, now, said), null);
        const drawer = Ready.drawerOf({
            eyebrow: "quest",
            title: "A Launcher That Fits",
            facts: [
                { heading: "Sessions" },
                { name: "Touched", at: now - 12 * 60_000, as: "came" },
                { name: "Id", said: "Y2YBTS", identifies: true, qualifier: "quest", hue: "alert" }
            ],
            body: "Rows served ready."
        }, now, said);
        Expect.same("a drawer is drawn with its facts worded", drawer, {
            title: "A Launcher That Fits",
            facts: [
                { heading: "Sessions" },
                { label: "Touched", value: "12 min ago" },
                { label: "Id", value: "Y2YBTS", qualifier: "quest", identifies: true, hue: "alert" }
            ],
            body: "Rows served ready.",
            eyebrow: "quest"
        });
    }

    function insides() {
        const held = {
            net: record("net", 1024),
            off: record("off", 2048),
            band: { key: "band", kind: "row", row: { parent: "net", order: 1024 } },
            ip: { key: "ip", kind: "row", row: { parent: "net", order: 512 } },
            "drawer:wifi:net": { key: "drawer:wifi:net", kind: "drawer",
                drawer: { title: "Lantern" } }
        };
        const keys = Ready.placed([], held, Object.keys(held), []);
        Expect.same("a place's own rows are those inside none", Ready.inside(keys, held, ""),
            ["net", "off"]);
        Expect.same("the rows inside one are those naming it, in their order",
            Ready.inside(keys, held, "net"), ["ip", "band"]);
        Expect.same("a row with none inside has none", Ready.inside(keys, held, "off"), []);
        Expect.same("a drawer served apart is read by the key its row names",
            Ready.drawerServed({ drawer: "drawer:wifi:net" }, held), { title: "Lantern" });
        Expect.same("a drawer named but not yet served is none yet",
            Ready.drawerServed({ drawer: "drawer:wifi:gone" }, held), null);
        Expect.same("a drawer of the row's own is its own",
            Ready.drawerServed({ drawer: { title: "Own" } }, held), { title: "Own" });
    }

    function acting() {
        const acts = [
            { key: "verb:wifi-leave", ask: { ask: "act", act: "leave" } },
            { key: "verb:copy:copy", copy: "10.0.0.2" },
            { key: "verb:open", open: "https://example.org" },
            { key: "verb:details", step: "drawer" },
            { key: "verb:more", step: "rows" },
            { key: "verb:spawn", run: ["sh"] },
            { ask: { ask: "act" } },
            { key: "verb:open", open: "" }
        ];
        Expect.same("each act is known by its form", acts.map(Ready.actForm),
            ["ask", "copy", "open", "step", "step", "", "", ""]);
        Expect.same("only the acts the shell can do are offered, in their order",
            Ready.actsOf({ acts: acts }).map(act => act.key), ["verb:wifi-leave",
                "verb:copy:copy", "verb:open", "verb:details", "verb:more"]);
        Expect.same("a row serving no acts offers none", Ready.actsOf({}), []);
        Expect.same("an act's verb is its words' where they say one",
            Ready.verbOf(acts[0], "disconnect"), "disconnect");
        Expect.same("else its form's", [1, 2, 3, 4].map(at => Ready.verbOf(acts[at], "")),
            ["copy", "open", "read", "more"]);
        const words = { noAnswer: "no answer", couldNot: "couldn't" };
        Expect.same("an ask done says nothing", Ready.refusal({ done: true }, words), "");
        Expect.same("one refused says the daemon's words",
            Ready.refusal({ error: { said: "not connected" } }, words), "not connected");
        Expect.same("one refused unworded says the launcher's",
            Ready.refusal({ error: { code: "NotJoined" } }, words), "couldn't");
        Expect.same("one unanswered says so", Ready.refusal(null, words), "no answer");
    }

    function ticking() {
        const now = new Date(2026, 10, 11, 14, 0).getTime();
        Expect.same("a row saying no time never ticks", Ready.ticks({ side: [{ said: "3" }] }),
            "");
        Expect.same("a row saying only days ticks as the day turns",
            Ready.ticks({ side: [{ at: now, as: "day" }, { at: now, as: "moment" }] }), "day");
        Expect.same("a row saying how long ago ticks each minute",
            Ready.ticks({ side: [{ at: now, as: "day" }, { at: now, as: "came" }] }), "minute");
        Expect.same("a drawer's time counts", Ready.ticks({
                drawer: { facts: [{ name: "Up", at: now, as: "lasted" }] }
            }), "minute");
        Expect.same("a note's time counts", Ready.ticks({ note: { at: now, as: "moment" } }),
            "day");
    }
}
