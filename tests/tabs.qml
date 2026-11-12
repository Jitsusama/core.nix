// Drives the launcher's Chrome tabs (modules/chrome/Tabs.qml) through
// Quickshell itself, with the real ReadyRows.qml making each row from the
// one the programs daemon serves ready (src/tab_rows.rs), told through its
// own Daemon.qml's took with no daemon to answer, and niri stood in. The
// pages are a collection, Tabs, each a row named by its title and found by
// its site, in the daemon's order, and its Return goes to the Chrome
// window it's in front of, else Chrome's newest. What's typed is ranked as
// the daemon answers (src/search.rs), the tabs holding its letters
// standing until then, and matched here only where it can't search. Each
// bookmark served (src/bookmark_rows.rs) is a row of the main search,
// found by its site too, and its Return runs Chrome with its address. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // A page's ready row, open in the window whose front tab is `front`.
    function row(address, title, front, order) {
        const key = `row:tabs:${address}`;
        const site = address.split("/")[2];
        return { key, kind: "row", row: {
            key, thing: `tab:${address}`, place: "tabs", kind: "tab", name: title,
            glyph: "web", hue: "program", side: [{ said: site, keep: 1 }], keywords: site,
            held: { address, app: "google-chrome", front, tabs: 1 },
            acts: [{ key: "verb:recent:site:go", go: "google-chrome" },
                { key: "verb:recent:site:link", copy: address }],
            order
        } };
    }

    // A bookmark's ready row, as the daemon serves it.
    function bookmark(address, name, order) {
        const key = `row:bookmarks:${address}`;
        const site = address.split("/")[2];
        return { key, kind: "row", row: {
            key, thing: `bookmark:${address}`, place: "bookmarks", kind: "bookmark", name,
            glyph: "web", hue: "program", side: [{ said: site, keep: 1 }],
            keywords: `${site} Work`, held: { address, app: "google-chrome", folder: "Work" },
            acts: [{ key: "verb:recent:site:open", open: address, in: "google-chrome" },
                { key: "verb:recent:site:link", copy: address }],
            order
        } };
    }

    QtObject {
        id: standIn

        property bool open: false
        function wants(source) {
            return open;
        }
    }

    Launcher.Tabs {
        id: tabs

        launcher: standIn
        settings: ({ name: "tabs" })
    }

    // Each time its search found more, as the daemon answers, and whether
    // it came fresh.
    property var foundMore: []
    Connections {
        target: tabs
        function onFoundMore(fresh) {
            root.foundMore = root.foundMore.concat([fresh]);
        }
    }

    // The names of what a search of the tabs gives.
    function searched(typed) {
        return tabs.search(typed).rows.map(made => made.name).join(", ");
    }

    // The cases run once each ReadyRows' Daemon.qml has made its link, which
    // its loader may make after the shell's own completion, and Quickshell
    // has read Chrome's entry, since a bookmark whose program is gone
    // offers no open.
    property bool begun: false
    Component.onCompleted: begin()
    Connections {
        target: tabs.served
        function onLinkChanged() {
            root.begin();
        }
    }
    Connections {
        target: tabs.bookmarks
        function onLinkChanged() {
            root.begin();
        }
    }
    Connections {
        target: DesktopEntries.applications
        function onValuesChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || !tabs.served.link || !tabs.bookmarks.link
            || DesktopEntries.byId("google-chrome") === null)
            return;
        begun = true;
        try {
            Niri.windows = {
                "7": { id: 7, app_id: "google-chrome", title: "Inbox - Google Chrome" },
                "9": { id: 9, app_id: "google-chrome", title: "core.nix - Google Chrome" },
                "4": { id: 4, app_id: "kitty", title: "Inbox" }
            };
            tabs.served.link.took([
                row("https://github.com/jitsusama/core.nix", "core.nix", "core.nix", 512),
                row("https://mail.google.com/mail/u/0/", "Inbox", "Inbox", 1024),
                row("https://docs.example.org/", "Notes", "Lost", 2048)
            ], []);
            tabs.bookmarks.link.took([
                bookmark("https://wiki.example.org/runbook", "Runbook", 1)
            ], []);
            standIn.open = true;
            Expect.expect("the tabs are a collection called Tabs",
                tabs.collection?.title === "Tabs", tabs.collection?.title);
            const names = tabs.everything().map(made => made.name).join(", ");
            Expect.expect("each page a row, in the daemon's order",
                names === "core.nix, Inbox, Notes", names);
            const marks = tabs.things.map(made => made.name).join(", ");
            Expect.expect("the bookmarks alone are the main search's", marks === "Runbook", marks);
            const words = tabs.everything().concat(tabs.things)
                .map(made => made.keywords?.text ?? "").join(", ");
            Expect.expect("each found by its site, a bookmark by its folder too",
                words === "github.com, mail.google.com, docs.example.org, wiki.example.org Work",
                words);
            const asked = tabs.served.link.asked;
            const waiting = searched("doc");
            Expect.expect("while the daemon's asked, the tabs holding the letters stand",
                waiting === "Notes", waiting);
            Expect.expect("what's typed is asked of the daemon",
                tabs.served.link.asked === asked + 1, tabs.served.link.asked - asked);
            tabs.served.answered("o", { id: 1, words: "o", marks: {}, keys: [
                "row:tabs:https://docs.example.org/", "row:tabs:https://github.com/jitsusama/core.nix"
            ] });
            const ranked = searched("o");
            Expect.expect("what's typed is ranked as the daemon answers it",
                ranked === "Notes, core.nix", ranked);
            Expect.expect("its answer has what's typed searched again",
                root.foundMore.length === 1, JSON.stringify(root.foundMore));
            tabs.served.answered("x", { id: 2, words: "x",
                error: { code: "unknown-place", message: "no place here" } });
            const matched = searched("inb");
            Expect.expect("a daemon that can't search leaves them matched here",
                matched === "Inbox", matched);
            tabs.things[0].run();
            Expect.expect("a bookmark's Return runs Chrome with its address",
                JSON.stringify(Niri.spawned)
                    === JSON.stringify([["google-chrome-stable", "https://wiki.example.org/runbook"]]),
                JSON.stringify(Niri.spawned));
            tabs.everything()[1].run();
            Expect.expect("Return goes to Chrome's window it's in front of",
                JSON.stringify(Niri.acted) === JSON.stringify([{ FocusWindow: { id: 7 } }]),
                JSON.stringify(Niri.acted));
            tabs.everything()[2].run();
            Expect.expect("and one in front of none, Chrome's newest",
                JSON.stringify(Niri.acted[1]) === JSON.stringify({ FocusWindow: { id: 9 } }),
                JSON.stringify(Niri.acted));
            Qt.callLater(() => Qt.exit(0));
        } catch (failed) {
            Expect.fail(failed.message);
        }
    }
}
