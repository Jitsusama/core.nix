// A notice's acts, in the order its card and its row in the launcher both
// list them, and how its drawer tells it, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-notice-is-one-thing` check
// copies Notice.js beside this file and runs
// `qml -platform offscreen notice.qml`.
import QtQuick
import "Notice.js" as Notice

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // A sender's actions as Quickshell gives them, cut to what's read.
    readonly property var slack: [
        { identifier: "default", text: "View" },
        { identifier: "reply", text: "Reply" }
    ]

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const said = acts => acts.map(act => act.identifier ? `${act.act}:${act.identifier}` : act.act);
        const open = { live: true, opens: Notice.opens(slack), offers: Notice.offers(slack) };

        // Still open: opening what it's about first, as Return does, by
        // its sender's own action rather than going to its window, then
        // what its sender offers, whether it's read, forgetting it, which
        // ends it, and copying last.
        expect("an open notice in the launcher",
            said(Notice.acts(open, { window: true, stack: false })),
            ["open", "offer:reply", "read", "forget", "copy"]);
        // On its card, the same, with what's done to the stack before the
        // copy, which is last everywhere.
        expect("the same notice on its card",
            said(Notice.acts(open, { window: true, stack: true })),
            ["open", "offer:reply", "read", "forget", "forgetAll", "all", "copy"]);
        // Its sender's own opening is Open, never listed as an offer.
        expect("what it's about isn't an offer", Notice.offers(slack),
            [{ identifier: "reply", text: "Reply" }]);
        // Gone from the screen, its sender's acts went with it, and it
        // can't be forgotten again: going to what it's about is going to
        // its sender's window.
        expect("a notice gone from the screen",
            said(Notice.acts({ live: false, opens: true, offers: Notice.offers(slack) },
                { window: true, stack: false })),
            ["goTo", "read", "copy"]);
        expect("nor anywhere to go once its window is closed",
            said(Notice.acts({ live: false, opens: false, offers: [] },
                { window: false, stack: false })),
            ["read", "copy"]);
        // One with nothing to open, on its card, whose Return forgets it:
        // forgetting it leads, once, and copying is never first.
        expect("a card with nothing to open",
            said(Notice.acts({ live: true, opens: false, offers: Notice.offers(slack) },
                { window: true, stack: true })),
            ["forget", "goTo", "offer:reply", "read", "forgetAll", "all", "copy"]);
        expect("the same among the notices, where Return steps in",
            said(Notice.acts({ live: true, opens: false, offers: [] },
                { window: false, stack: false })),
            ["read", "forget", "copy"]);

        // Each link in what it said is an act of its own, in the order it
        // said them, after what its sender offers and before Read, and
        // stays once the notice has gone, needing nothing of its sender.
        const links = [{ text: "#3", href: "https://github.com/x/y/pull/3" },
            { text: "the doc", href: "https://docs.google.com/d/1" }];
        const linked = acts => acts.map(act => act.href ? `link:${act.href}` : act.act);
        expect("its links, open",
            linked(Notice.acts(Object.assign({ links }, open), { window: true, stack: false })),
            ["open", "offer", "link:https://github.com/x/y/pull/3",
                "link:https://docs.google.com/d/1", "read", "forget", "copy"]);
        expect("and gone from the screen",
            linked(Notice.acts({ live: false, opens: true, offers: [], links },
                { window: false, stack: false })),
            ["link:https://github.com/x/y/pull/3", "link:https://docs.google.com/d/1", "read",
                "copy"]);

        // Its sender's own Read, where it offers one, by its identifier or
        // its words, which d sends at once on the card and in the launcher.
        const marking = { identifier: "mark-as-read", text: "Mark as Read" };
        expect("a sender's Read, by its identifier",
            Notice.readOffer(slack.concat([marking])), marking);
        expect("or by its words",
            Notice.readOffer([{ identifier: "2", text: "Read" }]), { identifier: "2", text: "Read" });
        expect("and none where it offers none, a word holding read among them",
            Notice.readOffer(slack.concat([{ identifier: "readme", text: "Readme" }])), null);

        // Its kind in words, from its category: a class alone or with its
        // subtype, an error failing, a bounced mail, and one never named
        // left out rather than shown raw.
        const named = {
            kinds: { im: "message", email: "mail", transfer: "transfer" },
            failed: "{kind} failed", bounced: "mail bounced"
        };
        expect("a kind by its class", Notice.kind("im", named), "message");
        expect("and with its subtype", Notice.kind("im.received", named), "message");
        expect("an error fails", Notice.kind("transfer.error", named), "transfer failed");
        expect("a bounced mail", Notice.kind("email.bounced", named), "mail bounced");
        expect("one never named", Notice.kind("x-gnome.music", named), "");
        expect("and none given", Notice.kind(undefined, named), "");

        // Its eyebrow: where it's from, then its kind, either alone.
        expect("from and kind", Notice.eyebrow("Slack", "message", "from {who}"),
            "from Slack \u00b7 message");
        expect("from alone", Notice.eyebrow("Desktop", "", "from {who}"), "from Desktop");
        expect("kind alone", Notice.eyebrow("", "mail", "from {who}"), "mail");

        // Its facts: Arrived first, unless Left says that time already.
        const arrived = { label: "Arrived", at: "10:51" };
        const urgency = { label: "Urgency", said: "low" };
        expect("arrived, left later, and its urgency",
            Notice.facts(arrived, { label: "Left", said: "put away at {at}", at: "10:53" },
                urgency),
            [{ label: "Arrived", value: "10:51" }, { label: "Left", value: "put away at 10:53" },
                { label: "Urgency", value: "low" }]);
        expect("left that same minute, Left stands for both",
            Notice.facts(arrived, { label: "Left", said: "put away at {at}", at: "10:51" },
                { label: "Urgency", said: "" }),
            [{ label: "Left", value: "put away at 10:51" }]);
        expect("still open, Left tells no time",
            Notice.facts(arrived, { label: "Left", said: "still on the screen", at: "" },
                { label: "Urgency", said: "" }),
            [{ label: "Arrived", value: "10:51" },
                { label: "Left", value: "still on the screen" }]);

        // Its row's name and note: its title, then the line of its body
        // that holds what was typed, its first line while nothing typed is
        // only in the body, and with no title its first line alone. A word
        // is held where it starts one of the text's words, as the search
        // finds it.
        const holds = (text, word) => text.toLowerCase().split(/\W+/)
            .some(one => one.startsWith(word));
        const log = "line 1 of the deploy went by\n\nline 7 of the deploy went by\n";
        expect("nothing typed", Notice.named("Deploy log", log, [], holds),
            { name: "Deploy log", note: "line 1 of the deploy went by" });
        expect("typed only in its title",
            Notice.named("Deploy log", log, ["deploy", "log"], holds),
            { name: "Deploy log", note: "line 1 of the deploy went by" });
        expect("typed in its body", Notice.named("Deploy log", log, ["log", "7"], holds),
            { name: "Deploy log", note: "line 7 of the deploy went by" });
        expect("the line holding most of it",
            Notice.named("Build", "it went by\nthe deploy failed", ["deploy", "fail"], holds),
            { name: "Build", note: "the deploy failed" });
        expect("no title", Notice.named("", "  first line\nsecond", ["sec"], holds),
            { name: "first line", note: "" });
        expect("no body", Notice.named("Battery low", "", ["low"], holds),
            { name: "Battery low", note: "" });

        // Lined, each row's note is worked out only once it's read, so a
        // search finding hundreds lines only the one the main search shows.
        let asked = 0;
        const counted = (text, word) => {
            asked += 1;
            return holds(text, word);
        };
        const rows = ["7", "1", "7"].map(line => ({
            key: line, title: "Deploy log", wrote: log, count: "", follows: "program:deploy"
        }));
        const lined = Notice.lined(rows, ["log", "7"], counted);
        expect("lined, no note is worked out", asked, 0);
        expect("until a note is read", lined[0].count, "line 7 of the deploy went by");
        const once = asked;
        void lined[0].count;
        expect("a note read twice is worked out once", asked, once);
        void lined[1].count;
        expect("another's was left till it was read", asked > once, true);
        expect("its row's own note stays its", rows.map(row => row.count), ["", "", ""]);
        expect("the rest is its row's", [lined[1].follows, lined[1].key],
            ["program:deploy", "1"]);
        lined[2].count = "said";
        expect("a note set holds", lined[2].count, "said");

        // A row lined, then given a field more, as History's age gives its
        // rows their side, keeps that field when it's lined again. Qt 6.12
        // names the slot of a getter's setter once per shape, so a field
        // turned into a getter on a shape grown since writes its setter
        // over the new field, or past the copy's end. That wrote over the
        // shell's memory until it crashed in Notice.lined, at 11:09 on
        // 2026-11-11.
        const grown = { key: "notice:grown", count: "", title: "Deploy log", wrote: log,
            atGrown: 1 };
        Notice.lined([grown], ["log"], holds);
        grown.sideGrown = "unread";
        const regrown = Notice.lined([grown], ["log"], holds)[0];
        expect("a field given between linings is kept",
            Object.assign({}, regrown).sideGrown, "unread");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
