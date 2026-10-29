// A notice's acts, in the order its card and its row in the launcher both
// list them, run in Qt's own JavaScript engine, the one the shell runs it
// in. The `a-notice-is-one-thing` check copies Notice.js beside this file
// and runs `qml -platform offscreen notice.qml`.
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

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
