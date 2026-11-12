// Drives the launcher's Chrome tabs (modules/chrome/Tabs.qml) through
// Quickshell itself, with the real ReadyRows.qml making each row from the
// one the programs daemon serves ready (src/tab_rows.rs), told through its
// own Daemon.qml's took with no daemon to answer, and niri stood in. Each
// page is a row named by its title and found by its site, in the daemon's
// order, and its Return goes to the Chrome window it's in front of, else
// Chrome's newest. It exits 0 once every case holds, and otherwise 1,
// saying which didn't.
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

    // The cases run once ReadyRows' Daemon.qml has made its link, which its
    // loader may make after the shell's own completion.
    property bool begun: false
    Component.onCompleted: begin()
    Connections {
        target: tabs.served
        function onLinkChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || !tabs.served.link)
            return;
        begun = true;
        try {
            Niri.windows = {
                "7": { id: 7, app_id: "google-chrome", title: "Inbox - Google Chrome" },
                "9": { id: 9, app_id: "google-chrome", title: "core.nix - Google Chrome" },
                "4": { id: 4, app_id: "kitty", title: "Inbox" }
            };
            tabs.served.link.took([
                row("https://github.com/jitsusama/core.nix", "core.nix", "core.nix", 2048),
                row("https://mail.google.com/mail/u/0/", "Inbox", "Inbox", 1024),
                row("https://docs.example.org/", "Notes", "Lost", 512)
            ], []);
            standIn.open = true;
            const names = tabs.things.map(made => made.name).join(", ");
            Expect.expect("each page a row, in the daemon's order",
                names === "core.nix, Inbox, Notes", names);
            const words = tabs.things.map(made => made.keywords?.text ?? "").join(", ");
            Expect.expect("each found by its site",
                words === "github.com, mail.google.com, docs.example.org", words);
            tabs.things[1].run();
            Expect.expect("Return goes to Chrome's window it's in front of",
                JSON.stringify(Niri.acted) === JSON.stringify([{ FocusWindow: { id: 7 } }]),
                JSON.stringify(Niri.acted));
            tabs.things[2].run();
            Expect.expect("and one in front of none, Chrome's newest",
                JSON.stringify(Niri.acted[1]) === JSON.stringify({ FocusWindow: { id: 9 } }),
                JSON.stringify(Niri.acted));
            Qt.callLater(() => Qt.exit(0));
        } catch (failed) {
            Expect.fail(failed.message);
        }
    }
}
