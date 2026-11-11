// How the shell takes a place's rows as its daemon serves them ready,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-launcher-words-each-ready-row-it-is-served` check copies
// launcher/ReadyRows.js, the When.js it reads from beside the launcher's
// folder, launcher/Ago.js, launcher/Interfaces.js and tests/lib/Expect.js
// beside this file and runs `qml -platform offscreen ready-rows.qml` over
// records made up here in the crate's wrapped form.
import QtQuick
import "launcher/ReadyRows.js" as Ready
import "launcher/Moved.js" as Moved
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
            on: "on {day}",
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
            moving();
            keywording();
            searching();
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
        // A day alone in a note, as the Home Network's last speed test is
        // said, the whole phrase the shell's, so today never reads "on today".
        Expect.same("on a day today", say({ at: at(11, 9, 5), as: "on" }), "today");
        Expect.same("on a day yesterday", say({ at: at(10, 9, 5), as: "on" }), "yesterday");
        Expect.same("on a day this week", say({ at: at(5, 10, 52), as: "on" }), "on Thursday");
        Expect.same("on a day this year", say({ at: new Date(2026, 9, 28).getTime(), as: "on" }),
            "on 28 Oct");
        Expect.same("on a day another year",
            say({ at: new Date(2025, 10, 13).getTime(), as: "on" }), "on 13 Nov 2025");
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
        Expect.same("each item is kept apart with its width and its hue", Ready.sideItems([
                { said: "71 \u00b0C", keep: 3 }, { said: "29 \u00b0C below high", hue: "warning" },
                { said: "" }
            ], now, said), [{ said: "71 \u00b0C", keep: 3 },
                { said: "29 \u00b0C below high", hue: "warning" }]);
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
        Expect.same("a note of items is said as a side is", Ready.noteSaid([{ said: "11.9 Mb/s up" },
            { at: now - 86_400_000, as: "on" }, { said: "" }, { said: "1 outage" }], now, said),
            "11.9 Mb/s up \u00b7 yesterday \u00b7 1 outage");
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
        // A budget's: its eyebrow names it as written, a fact says words,
        // and one carries its meter.
        const meter = { shape: "stroke", values: [0.4], warn: false };
        Expect.same("a drawer naming the world keeps its meters and words", Ready.drawerOf({
            eyebrow: "Personal",
            eyebrowNames: true,
            title: "This Week",
            facts: [
                { name: "Used", said: "$40 of $100", figure: meter },
                { name: "Covers", said: "Personal work", words: true }
            ]
        }, now, said), {
            title: "This Week",
            facts: [
                { label: "Used", value: "$40 of $100", figure: meter },
                { label: "Covers", value: "Personal work", words: true }
            ],
            body: "",
            eyebrow: "Personal",
            eyebrowNames: true
        });
        // A Slack message's: its body is prose, as served, and a drawer
        // saying anything else of it isn't.
        Expect.same("a drawer of prose says so", Ready.drawerOf({
            title: "Ada Lovelace",
            body: "can you check the build",
            prose: true
        }, now, said), {
            title: "Ada Lovelace",
            facts: [],
            body: "can you check the build",
            prose: true
        });
        Expect.same("only a drawer served as prose is", Ready.drawerOf({
            title: "Ada Lovelace",
            prose: "yes"
        }, now, said), { title: "Ada Lovelace", facts: [], body: "" });
        const items = [{ said: "in ~/Videos/Recordings", keep: 2, path: "~/Videos/Recordings" },
            { said: "video", keep: 1 }];
        const shown = Ready.drawerOf({
            title: "take.mp4",
            facts: [],
            preview: { picture: "/run/frame.png", across: 640, down: 360 },
            eyebrowItems: items,
            eyebrowNames: true
        }, now, said);
        Expect.same("a drawer's picture and its eyebrow's parts pass through as served", shown, {
            title: "take.mp4",
            facts: [],
            body: "",
            preview: { picture: "/run/frame.png", across: 640, down: 360 },
            eyebrowItems: items,
            eyebrowNames: true
        });
        Expect.same("nothing served of them is none",
            Object.keys(Ready.drawerOf({ title: "plain" }, now, said)), ["title", "facts", "body"]);
        Expect.same("a notice's drawer keeps how its words are drawn", Ready.drawerOf({
            eyebrow: "Slack", eyebrowNames: true, title: "Ana", body: "<b>hi</b>",
            styled: true, prose: true, picture: "file:///p.png", facts: []
        }, now, said), { title: "Ana", facts: [], body: "<b>hi</b>", eyebrow: "Slack",
            styled: true, picture: "file:///p.png", eyebrowNames: true, prose: true });
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

        // What a row's parts are made of, so a closing makes again only the
        // spots of a row whose stamp moved (launcher/Spots.js).
        const stamp = Ready.stampOf(keys, held, "net");
        Expect.same("a row's stamp holds while nothing it's made of changes",
            Ready.stampOf(keys, held, "net"), stamp);
        const banded = Object.assign({}, held, {
            band: { key: "band", kind: "row", row: { parent: "net", order: 1024, name: "5 GHz" } }
        });
        Expect.same("a part inside it changing moves it",
            Ready.stampOf(keys, banded, "net") === stamp, false);
        const drawn = Object.assign({}, held, { net: { key: "net", kind: "row",
            row: { thing: "quest:net", order: 1024, drawer: "drawer:wifi:net" } } });
        const drawnStamp = Ready.stampOf(keys, drawn, "net");
        drawn["drawer:wifi:net"] = { key: "drawer:wifi:net", kind: "drawer",
            drawer: { title: "Lantern 5G" } };
        Expect.same("so does the drawer it names apart",
            Ready.stampOf(keys, drawn, "net") === drawnStamp, false);
        Expect.same("another row changing leaves it",
            Ready.stampOf(keys, Object.assign({}, held, { off: record("off", 4096) }), "net"),
            stamp);
        Expect.same("a key holding no row has none", Ready.stampOf(keys, held, "drawer:wifi:net"),
            null);
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
            { key: "verb:open", open: "" },
            { key: "verb:notice:goTo", go: "Slack" },
            { key: "verb:notice:goTo", go: "" }
        ];
        Expect.same("each act is known by its form", acts.map(Ready.actForm),
            ["ask", "copy", "open", "step", "step", "", "", "", "go", ""]);
        Expect.same("only the acts the shell can do are offered, in their order",
            Ready.actsOf({ acts: acts }).map(act => act.key), ["verb:wifi-leave",
                "verb:copy:copy", "verb:open", "verb:details", "verb:more",
                "verb:notice:goTo"]);
        const going = acts[8];
        const slack = { id: "Slack" };
        Expect.same("going to a program with a window open goes to its window",
            Ready.going(going, { slack: { id: 9 } }, slack), { window: 9 });
        Expect.same("with only its entry, it opens the program",
            Ready.going(going, { foot: { id: 7 } }, slack), { entry: slack });
        Expect.same("with neither, Return does nothing", Ready.going(going, {}, null), null);
        const roles = { agent: "A", bell: "B" };
        Expect.same("a mark is its role's glyph, or its own code point served in hex",
            ["agent", { code: "f198" }, "absent", { code: "" }, { code: "zz" }, null]
                .map(glyph => Ready.glyphOf(glyph, roles)),
            ["A", "\uf198", "", "", "", ""]);
        const worded = { name: "Open the Link", glyph: "L" };
        Expect.same("an act is said in its key's words where it's served none",
            Ready.actWorded(acts[0], worded, roles),
            { name: "Open the Link", glyph: "L", side: "" });
        Expect.same("and in its sender's where it is", Ready.actWorded({
            key: "verb:notice:reply", name: "Reply", glyph: "agent" }, worded, roles),
            { name: "Reply", glyph: "A", side: "" });
        const reads = { ask: "mark", notices: [7], read: true };
        Expect.same("an act the shell does sends its also once done", [
            { key: "verb:notice:goTo", go: "slack", also: reads },
            { key: "verb:open", open: "https://example.org", also: reads },
            { key: "verb:copy", copy: "hi", also: reads }].map(Ready.alsoOf),
            [reads, reads, reads]);
        Expect.same("an ask, a step or an act with none sends nothing besides", [
            { key: "verb:a", ask: { ask: "act" }, also: reads },
            { key: "verb:more", step: "rows", also: reads },
            { key: "verb:copy", copy: "hi" }].map(Ready.alsoOf), [null, null, null]);
        Expect.same("a link says its host at its side", Ready.actWorded({
            key: "verb:notice:link", side: "github.com" }, worded, roles),
            { name: "Open the Link", glyph: "L", side: "github.com" });
        const linkTo = href => ({ key: "verb:notice:link", open: href });
        const repeated = {};
        Expect.same("a key a row holds once stays bare, and two alike go by their targets",
            Ready.actKeys([linkTo("https://a.org"), linkTo("https://b.org"),
                { key: "verb:notice:read", ask: { ask: "mark" } }], repeated),
            ["verb:notice:link:https://a.org", "verb:notice:link:https://b.org",
                "verb:notice:read"]);
        Expect.same("the first link gone, the second keeps its key, so the cursor stays on it",
            Ready.actKeys([linkTo("https://b.org")], repeated), ["verb:notice:link:https://b.org"]);
        Expect.same("two alike with no target of their own go by their place",
            Ready.actKeys([{ key: "verb:more", step: "rows" },
                { key: "verb:more", step: "rows" }], {}), ["verb:more:0", "verb:more:1"]);
        Expect.same("another row's keys are its own", Ready.actKeys([linkTo("https://b.org")], {}),
            ["verb:notice:link"]);
        const asking = { key: "verb:notice:open", ask: { ask: "invoke", id: 4 }, token: true };
        Expect.same("an ask that takes a token is sent with it",
            Ready.tokened(asking, "abc"), { ask: "invoke", id: 4, token: "abc" });
        Expect.same("and never without one", [Ready.tokened(asking, ""),
            Ready.tokened(asking, undefined)], [null, null]);
        Expect.same("an ask that takes none is sent as served", Ready.tokened(acts[0], "abc"),
            acts[0].ask);
        Expect.same("a row serving no acts offers none", Ready.actsOf({}), []);
        // A document opens in Google's window, not the browser, so its
        // open names the program it opens in (Y2YBTS, de23ba21).
        const link = "https://docs.google.com/document/d/d1/edit";
        const inGoogle = { key: "verb:docs:open", open: link, in: "google-opens" };
        const inGone = { key: "verb:docs:open", open: link, in: "gone" };
        const has = id => id === "google-opens";
        Expect.same("an open in a program here is offered, one in a program gone isn't",
            Ready.actsOf({ acts: [inGoogle, inGone] }, has).length, 1);
        Expect.same("and it opens its link in that program, the link after its command",
            Ready.opening(inGoogle, id => (has(id) ? ["/bin/google-opens"] : null)),
            ["/bin/google-opens", link]);
        Expect.same("never elsewhere, once its program's gone",
            Ready.opening(inGone, () => null), null);
        // Its program gone between listing and Return, it opens nothing
        // and says why, never failing silently (Y2YBTS on #3313).
        const becoming = { missing: "{program} is gone" };
        Expect.same("an open whose program is here runs its opening, saying nothing",
            Ready.opened(inGoogle, () => ["/bin/google-opens"], becoming),
            { run: ["/bin/google-opens", link], why: "" });
        Expect.same("one whose program went runs nothing and says why, naming it as it was shown",
            Ready.opened(Object.assign({ program: "Google" }, inGone), () => null, becoming),
            { run: null, why: "Google is gone" });
        Expect.same("an open naming none opens as any link",
            Ready.opening({ key: "verb:open", open: link }, () => null), ["xdg-open", link]);
        Expect.same("an act's verb is its words' where they say one",
            Ready.verbOf(acts[0], "disconnect"), "disconnect");
        Expect.same("else its form's", [1, 2, 3, 4].map(at => Ready.verbOf(acts[at], "")),
            ["copy", "open", "read", "more"]);
        Expect.same("an act named by its daemon says that name, as the foot says verbs",
            Ready.verbOf({ key: "sound:mute", name: "Unmute", ask: {} }, "mute"), "unmute");
        const words = { noAnswer: "no answer", couldNot: "couldn't" };
        Expect.same("an ask done says nothing", Ready.refusal({ done: true }, words), "");
        Expect.same("one refused says the daemon's words",
            Ready.refusal({ error: { said: "not connected" } }, words), "not connected");
        Expect.same("one refused with only a message for the journal says the launcher's",
            Ready.refusal({ error: { code: "busy", message: "spend-watch can't act yet" } },
                words), "couldn't");
        Expect.same("one refused unworded says the launcher's",
            Ready.refusal({ error: { code: "NotJoined" } }, words), "couldn't");
        Expect.same("one unanswered says so", Ready.refusal(null, words), "no answer");
        closing();
    }

    function closing() {
        const paste = { key: "verb:copy:paste", ask: { ask: "paste", copy: 3 }, closes: true };
        const both = Object.assign({ becomes: { key: "row:copies:3" } }, paste);
        Expect.same("an ask that shuts the launcher is still an ask", Ready.actForm(paste),
            "ask");
        Expect.same("and shuts it", Ready.closes(paste), true);
        Expect.same("an ask that doesn't, doesn't",
            Ready.closes({ key: "verb:wifi-leave", ask: { ask: "act" } }), false);
        Expect.same("only an ask shuts it",
            Ready.closes({ key: "verb:copy:copy", copy: "a", closes: true }), false);
        Expect.same("one that shuts it is done in Return's turn, the launcher let go",
            Ready.leaving(paste), { stays: false, now: true, later: false });
        Expect.same("one that asks without shutting it keeps it, to show what it became",
            Ready.leaving({ key: "verb:wifi-leave", ask: { ask: "act" } }),
            { stays: true, now: false, later: false });
        Expect.same("one that brings its sender forward is done while the keys are held",
            Ready.leaving({ key: "verb:notice:open", ask: { ask: "invoke" }, token: true }),
            { stays: false, now: true, later: false });
        Expect.same("one that copies leaves it as a value's row does",
            Ready.leaving({ key: "verb:copy:copy", copy: "a" }),
            { stays: false, now: false, later: false });
        Expect.same("going to a program is done once the launcher's gone",
            Ready.leaving({ key: "verb:go", go: "kitty" }),
            { stays: false, now: false, later: true });
        Expect.same("an ask is read by its act's words in the key language",
            Ready.wordsOf({ key: "verb:firmware:install", ask: { ask: "install" } }),
            "verb:firmware:install");
        Expect.same("but one that shuts the launcher isn't, so Return does it from the find",
            Ready.wordsOf(paste), "");
        Expect.same("nor any other act", Ready.wordsOf({ key: "verb:copy:copy", copy: "a" }), "");
        Expect.same("shutting it and saying what it becomes is refused", Ready.misformed(both),
            "closes and becomes");
        Expect.same("so it's no act at all", Ready.actForm(both), "");
        Expect.same("and isn't offered", Ready.actsOf({ acts: [both, paste] }).map(act => act.key),
            ["verb:copy:paste"]);
        Expect.same("a well-formed act says nothing", Ready.misformed(paste), "");
    }

    // Whether a row told again moved only, as a daemon making room between
    // two rows tells every row it moved, so what was made of it stands.
    function moving() {
        const was = { thing: "sound:a", name: "Speakers", order: 1024,
            side: [{ said: "50%" }], held: { volume: 0.5, muted: false } };
        const moved = Object.assign({}, was, { order: 1536 });
        Expect.same("a row whose order alone moved moved only", Moved.movedOnly(was, moved),
            true);
        Expect.same("one told alike moved only too", Moved.movedOnly(was,
            JSON.parse(JSON.stringify(was))), true);
        Expect.same("one whose words moved did more", Moved.movedOnly(was,
            Object.assign({}, moved, { side: [{ said: "60%" }] })), false);
        Expect.same("as one whose held facts moved did", Moved.movedOnly(was,
            Object.assign({}, moved, { held: { volume: 0.5, muted: true } })), false);
        Expect.same("and one that gained a field", Moved.movedOnly(was,
            Object.assign({}, moved, { note: "new" })), false);
        Expect.same("or lost one", Moved.movedOnly(was, { thing: "sound:a", name: "Speakers",
            order: 1536, side: [{ said: "50%" }] }), false);
        Expect.same("a list is not an object of its indexes", Moved.movedOnly({ side: ["a"] },
            { side: { 0: "a" } }), false);
        Expect.same("a row never made moved nowhere", Moved.movedOnly(undefined, moved), false);
    }

    // What a served row is found by beyond its name, from the daemon and
    // its place, in one text the launcher makes ready to match once.
    function keywording() {
        Expect.same("a row with none is found by none", Ready.keywordsOf(undefined, undefined),
            "");
        Expect.same("a daemon's list is its words in turn",
            Ready.keywordsOf(["grinning", "face"], undefined), "grinning face");
        Expect.same("a place's text is searched, as a screen's name is",
            Ready.keywordsOf(undefined, "Scale 100%"), "Scale 100%");
        Expect.same("both are, the daemon's first", Ready.keywordsOf("radio", "DP-1"),
            "radio DP-1");
        Expect.same("words a place made ready already are read for their text",
            Ready.keywordsOf(null, { text: "eDP-1", mask: 3 }), "eDP-1");
        Expect.same("a list's other values say nothing",
            Ready.keywordsOf(["fan", 3, null], ""), "fan");
    }

    // A search's answer, its found rows the watch never served among them
    // (ready-rows.md's search, RTOIMQ, d482d399).
    function searching() {
        const none = Ready.foundOf("shot", null);
        Expect.same("no answer yet finds nothing, for no words", [none.words, none.keys],
            [null, []]);
        const found = Ready.foundOf("shot", {
            keys: ["b", "far", "lost"],
            marks: { far: [[0, 4]] },
            rows: { far: { thing: "quest:far", name: "Shot far" } }
        });
        Expect.same("an answer holds its words, its order and its marks",
            [found.words, found.keys, found.marks], ["shot", ["b", "far", "lost"],
                { far: [[0, 4]] }]);
        const held = { b: record("b", 2048) };
        Expect.same("a row it found that the watch serves is the watch's",
            Ready.foundRecord(found, held, "b"), held.b);
        Expect.same("one the watch never served is the answer's, read as any record",
            Ready.rowOf(Ready.foundRecord(found, held, "far")),
            { thing: "quest:far", name: "Shot far" });
        held.far = record("far", 1024);
        Expect.same("and a change naming it since replaces the answer's",
            Ready.foundRecord(found, held, "far"), held.far);
        Expect.same("a key served by neither is none", Ready.foundRecord(found, held, "lost"),
            null);
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
        Expect.same("a note's items count, a day alone turning with the day",
            Ready.ticks({ note: [{ said: "1 outage" }, { at: now, as: "on" }] }), "day");
    }
}
