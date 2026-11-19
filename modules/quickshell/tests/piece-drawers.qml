// What Now's own pieces say in their drawers, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-piece-says-all-it-knows`
// check copies pieces/Drawers.js, with the launcher/Program.js it reads,
// beside this file and runs `qml -platform offscreen piece-drawers.qml`
// over what the disk guards, Notices, lag-watch, niri, the recorder and
// Quiet say, shaped as each piece hands it over.
import QtQuick
import "pieces/Drawers.js" as Drawers

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

    // Each fact named by its id, so a fact's order is read off.
    function named(id) {
        return `[${id}]`;
    }

    // A drawer's facts, each as its name and value, to compare at once.
    function said(drawer) {
        return drawer.facts.map(fact => `${fact.label} ${fact.value}`);
    }

    property var failures: []
    property int asked: 0

    function expect(what, got, wanted) {
        asked += 1;
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    function disks() {
        const told = {
            now: 1000,
            named,
            said: {
                eyebrow: "on this machine \u00b7 disk",
                hourly: "{amount} an hour",
                grew: "{path} +{amount}",
                cleared: "{amount} from {path} at {time}"
            },
            shown: bytes => `${bytes} B`,
            duration: seconds => `${seconds} s`,
            within: path => path.replace("/home/joel", "~"),
            time: at => `@${at}`
        };
        const worst = {
            path: "/home",
            free: 40,
            size: 500,
            full_at: 4600,
            fill_per_hour: 12,
            says: "Nix's cache is filling it."
        };
        const holders = [
            { path: "/home/joel/src", grew: { bytes: 5 } },
            { path: "/home/joel/.cache/nix", grew: { bytes: 90 } },
            { path: "/home/joel/.local/share", grew: { bytes: 30 } },
            { path: "/home/joel/Music", grew: { bytes: 0 } },
            { path: "/home/joel/Videos", grew: { bytes: 2 } }
        ];
        const cleared = [{ at: 900, path: "/home/joel/.cache/nix", bytes: 70 }];
        const drawer = Drawers.disk({ worst, holders, cleared }, told);
        expect("a disk is named by its path, says what moves first, its room left to the row",
            [drawer.title, drawer.eyebrow].concat(said(drawer), drawer.body), [
                "/home", "on this machine \u00b7 disk",
                "[machine:disk:full-in] 3600 s",
                "[on:disk:filling] 12 B an hour",
                "[on:disk:growing] ~/.cache/nix +90 B \u00b7 ~/.local/share +30 B \u00b7 "
                    + "~/src +5 B",
                "[machine:disk:size] 500 B",
                "[on:disk:cleared] 70 B from ~/.cache/nix at @900",
                "Nix's cache is filling it."
            ]);
        expect("each holder growing is an item the drawer wraps between, its path apart",
            drawer.facts[2].valueItems.slice(0, 2), [
                { said: "~/.cache/nix +90 B", path: "~/.cache/nix" },
                { said: "~/.local/share +30 B", path: "~/.local/share" }
            ]);
        const steady = Drawers.disk({
            worst: { path: "/", free: 7, total: 9, full_at: 600, fill_per_hour: -3 },
            holders: [{ path: "/nix", grew: { bytes: 0 } }]
        }, told);
        expect("a disk emptying, full in the past or growing nowhere says none of it",
            said(steady), ["[machine:disk:size] 9 B"]);
        expect("no disk the guards name, no drawer", Drawers.disk({ worst: null }, told), null);
    }

    function waiting() {
        const drawer = Drawers.notices([
            { app: "Slack", at: 5 },
            { app: "Calendar", at: 4 },
            { app: "Slack", at: 3 },
            { app: "Slack", at: 1 }
        ], { eyebrow: "on this desktop \u00b7 notices" });
        expect("each sender is a fact with its count, the last to send first, the total unsaid",
            [drawer.eyebrow].concat(said(drawer)),
            ["on this desktop \u00b7 notices", "Slack 3", "Calendar 1"]);
    }

    function lagged() {
        const told = {
            named,
            said: {
                eyebrow: "on this machine \u00b7 lag",
                over: "{share} of the last {seconds} s",
                "stall:cpu": "processor",
                "stall:memory": "memory",
                "stall:io": "disk"
            },
            tells: {
                desktop: "The desktop waited on {what}.",
                programs: "Programs waited on {what}."
            }
        };
        const window = {
            seconds: 10.02,
            stalls: {
                session: { cpu_some: 0, memory_some: 12.345, io_some: 3 },
                app: { cpu_some: 41.06, memory_some: 0.5, io_some: 100 },
                jobs: { cpu_some: 0.04, memory_some: 0, io_some: 0 }
            },
            memory: { session: { swap_mib: 1024 }, app: { swap_mib: 120 } },
            worst: "memory",
            who: "apps",
            trouble: ["apps stalled on memory 0.20% of the time"]
        };
        const drawer = Drawers.lag(window, "memory", told);
        expect("lag says how long the most any slice waited, of what window, each slice's"
            + " stalls to three figures, too little to say giving way, the swap, then who"
            + " waited on what in our own sentence",
            [drawer.eyebrow].concat(said(drawer), drawer.body), [
                "on this machine \u00b7 lag",
                "[on:lag:waited] 12.3% of the last 10 s",
                "[on:lag:session] memory 12.3% \u00b7 disk 3.00%",
                "[on:lag:app] processor 41.1% \u00b7 memory 0.500% \u00b7 disk 100%",
                "[on:lag:swap] 1.20\u00a0GB",
                "Programs waited on memory."
            ]);
        expect("the desktop waiting says so", Drawers.lag(Object.assign({}, window,
            { who: "desktop" }), "memory", told).body, "The desktop waited on memory.");
        const days = { day: "a day", days: "{count} days" };
        expect("how long is said in its two largest units to the minute, never as a clock",
            [21_420, 2_400, 7_200, 30, 86_400, 3 * 86_400].map(seconds => Drawers.span(seconds,
                days)), ["5 h 57 min", "40 min", "2 h", "1 min", "a day", "3 days"]);
        expect("nothing waiting long and no swap says only the slices lag-watch watched",
            said(Drawers.lag({ stalls: { build: { cpu_some: 1, memory_some: 2, io_some: 0 } } },
                "", told)),
            ["[on:lag:build] processor 1.00% \u00b7 memory 2.00%"]);
    }

    function shared() {
        const told = {
            named,
            held: true,
            said: {
                eyebrow: "on this desktop \u00b7 screen share",
                several: "{count} things",
                held: "held",
                shown: "still show",
                window: "window",
                screen: "screen"
            }
        };
        const window = Drawers.sharing([{ target: { Window: { id: 4 } }, stream_id: 61 }],
            ["Pull request #12"], told);
        expect("one share is its title, then the notices, its kind and its stream, last",
            [window.eyebrow, window.title].concat(said(window)), [
                "on this desktop \u00b7 screen share", "Pull request #12",
                "[on:sharing:notices] held", "[on:sharing:kind] window",
                "[on:sharing:stream] 61"
            ]);
        expect("the stream is an identifier", window.facts[2].identifies, true);
        const screen = Drawers.sharing([{ target: { Output: { name: "eDP-1" } } }],
            ["the whole screen"], Object.assign({}, told, { held: false }));
        expect("the whole screen, its notices showing, with no stream yet, says no stream",
            [screen.title].concat(said(screen)),
            ["the whole screen", "[on:sharing:notices] still show", "[on:sharing:kind] screen"]);
        const several = Drawers.sharing([{ target: {} }, { target: {} }],
            ["the whole screen", "Notes"], told);
        expect("several shares say how many, then each",
            [several.title].concat(said(several)),
            ["2 things", "[on:sharing:what] the whole screen", "[on:sharing:what] Notes"]);
    }

    function recorded() {
        const drawer = Drawers.recording("3:41", "a window", {
            named,
            said: { eyebrow: "in {where} \u00b7 recording" },
            where: "~/Videos/Recordings"
        });
        expect("a recording is where it lands, then how long and what it shows",
            [drawer.eyebrow].concat(said(drawer)), [
                "in ~/Videos/Recordings \u00b7 recording", "[capture:length] 3:41",
                "[on:recording:showing] a window"
            ]);
    }

    function quieted() {
        const told = {
            named,
            said: {
                eyebrow: "on this desktop \u00b7 do not disturb",
                one: "1 notice",
                many: "{count} notices"
            },
            tells: {
                kept: "Turned on by hand.",
                shared: "On while the screen is shared.",
                off: "New notices wait in the notices list rather than showing."
            }
        };
        told.said.until = "until {time}";
        told.said.sharing = "comes on by itself";
        told.said.unshared = "stays off";
        const kept = Drawers.quiet({ since: "13:05", held: 1, kept: true }, told);
        expect("do not disturb says since when and what it held, why last, never until when",
            [kept.eyebrow].concat(said(kept), kept.body), [
                "on this desktop \u00b7 do not disturb", "[on:quiet:since] 13:05",
                "[on:quiet:held] 1 notice", "Turned on by hand."
            ]);
        const sharing = Drawers.quiet({ since: "", held: 4, kept: false }, told);
        expect("on for a share, with no start known, says what it held and why",
            said(sharing).concat(sharing.body),
            ["[on:quiet:held] 4 notices", "On while the screen is shared."]);
        // Off, Desktop's row says what Return would do and until when, and whether a
        // share turns it on, so it says more than its line (Joel, 2026-11-11: every row
        // has a drawer).
        const off = Drawers.quiet({ off: true, hour: "14:05", whileSharing: true }, told);
        expect("off, it says until when Return holds them, what a share does, and what it does",
            [off.eyebrow].concat(said(off), off.body), [
                "on this desktop \u00b7 do not disturb", "[session:quiet:60] until 14:05",
                "[on:quiet:sharing] comes on by itself",
                "New notices wait in the notices list rather than showing."
            ]);
        const unshared = Drawers.quiet({ off: true, hour: "14:05", whileSharing: false }, told);
        expect("off, with a share left to Joel, it says a share leaves it off",
            said(unshared), ["[session:quiet:60] until 14:05", "[on:quiet:sharing] stays off"]);
    }

    function run() {
        disks();
        waiting();
        lagged();
        shared();
        recorded();
        quieted();

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
