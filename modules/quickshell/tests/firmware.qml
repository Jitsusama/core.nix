pragma ComponentBehavior: Bound
// Drives the launcher's Firmware (modules/quickshell/launcher/Firmware.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving a device's Install ready (the ready-row form, as
// firmware_rows.rs makes it), with the launcher stood in, open, "firmware"
// typed. Install's step asking if Joel's sure asks the daemon's install
// once, and the row it becomes at once, then the step the daemon serves
// while it installs, offer nothing to run: a Return on either only closes
// (Launcher.qml's run), so a second Return never installs again, as the
// desktop test's dropped case held (#5869's F250 and F261). It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first rows"

    // The launcher as Firmware reads it: open, "firmware" typed, so its
    // rows can be seen and its daemon is watched.
    QtObject {
        id: theLauncher

        property bool open: true
        property string typed: "firmware"
        property var answer: null
        property var held: []
        property var matches: []
        property var placed: []

        // No frame is drawn here, so what waits on one runs at once.
        function afterFrame(then) {
            then();
        }

        // The row an act says each of its rows becomes, by its key, as the
        // launcher draws it.
        property var becoming: ({})
        function becomesAll(provider, holds) {
            for (const one of holds)
                becoming[one.key] = one.row;
        }
    }

    // A row as the machine daemon serves it, in the crate's wrap, without
    // it as an act's `becomes` holds it.
    function made(id, fields) {
        const key = `row:firmware:${id}`;
        return Object.assign({ key: key, thing: `firmware:${id}`, place: "firmware",
            kind: "act", depth: 0, side: [], acts: [] }, fields);
    }
    function served(row) {
        return { key: row.key, kind: "row", row: row };
    }
    readonly property var more: ({ key: "verb:more", step: "rows" })
    readonly property var device: served(made("b10", {
            kind: "device",
            order: 1,
            name: "System Firmware",
            glyph: "firmware",
            side: [{ said: "1.13.0 from 1.12.0" }],
            acts: [root.more]
        }))
    function install(note) {
        return served(made("b10:install", {
            parent: "row:firmware:b10",
            depth: 1,
            order: 1,
            name: "Install 1.13.0",
            hue: "action",
            note: note,
            acts: [root.more]
        }));
    }
    // The step asking if Joel's sure, as it reads while it installs:
    // in its place, saying so, asking nothing.
    readonly property var sureGoing: made("b10:install:sure", {
            parent: "row:firmware:b10:install",
            depth: 2,
            order: 1,
            name: "Install 1.13.0 on System Firmware?",
            hue: "alert",
            note: "installing"
        })
    readonly property var sure: served(made("b10:install:sure", {
            parent: "row:firmware:b10:install",
            depth: 2,
            order: 1,
            name: "Install 1.13.0 on System Firmware?",
            hue: "alert",
            note: "final",
            sure: true,
            acts: [{ key: "verb:firmware:install", ask: { ask: "act", act: "firmware-install",
                        device: "b10", patience_s: 600 }, becomes: root.sureGoing }]
        }))
    readonly property var head: ({ key: "head:firmware", kind: "head", head: {
                said: "an update for System Firmware" } })

    // The shell's watch, so the daemon can tell it more, and each install
    // it asked.
    property var watch: null
    property var installs: []

    // A stand-in for the machine daemon, speaking the daemon interface:
    // hello, a watch of Firmware's rows and headline answered whole, and an
    // install taken, the rows then told as they read while it goes.
    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        handler: Socket {
            id: socket

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer;
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "machine",
                            state: "running" };
                    } else if (ask.ask === "watch") {
                        root.watch = { socket: socket, id: ask.id };
                        answer = { id: ask.id, records: [root.sure, root.install(""),
                                root.head, root.device] };
                    } else if (ask.ask === "act" && ask.act === "firmware-install") {
                        root.installs = root.installs.concat([ask.device]);
                        answer = { id: ask.id, ok: true };
                        Watch.tell(root.watch, { changed: [root.install("installing"),
                                root.served(root.sureGoing)] });
                    } else {
                        answer = { id: ask.id, ok: true };
                    }
                    socket.write(`${JSON.stringify(answer)}\n`);
                    socket.flush();
                }
            }
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
    }

    // Firmware says it answered as its first rows come, and each time the
    // daemon tells it something while it shows; each case waits for what
    // it needs to have been heard.
    Launcher.Firmware {
        id: theFirmware

        launcher: theLauncher
        settings: ({ name: "firmware", fwupdmgr: "/run/fwupdmgr" })
        onAnswered: {
            try {
                if (root.waiting === "the first rows")
                    root.first();
                else if (root.waiting === "the install going")
                    root.going();
            } catch (error) {
                root.waiting = "a failure";
                Expect.fail(error.message);
            }
        }
    }

    // The step asking if Joel's sure, a step into Install, a step into
    // the device.
    function sureStep() {
        const answer = theFirmware.answer("firmware");
        const install = answer?.rows?.[0]?.verbs?.()?.[0];
        return { install: install, sure: install?.verbs?.()?.[0] };
    }

    // What a Return would do with a row, as Launcher.qml's run reads it:
    // its own act, a function to run, words to ask, or nothing, closing.
    function offers(row) {
        return [row?.returns ? "returns" : "", typeof row?.run === "function" ? "run" : "",
            row?.given ? "given" : "", row?.sure === true ? "sure" : ""].join("");
    }

    // Install is ready: its step asks if Joel's sure, and its Return asks
    // the install, its row drawn at once as installing, asking nothing.
    function first() {
        const { install, sure } = sureStep();
        if (sure === undefined)
            return;
        root.waiting = "the install going";
        Expect.expect("Install a step into the device, its sure step a step into it",
            install.name === "Install 1.13.0"
            && sure.name === "Install 1.13.0 on System Firmware?",
            `${install?.name} ${sure?.name}`);
        Expect.expect("the sure step's Return does its act", offers(sure) === "runsure",
            offers(sure));
        theLauncher.answer = theFirmware.answer("firmware");
        sure.run();
        const becomes = theLauncher.becoming["firmware:b10:install:sure"];
        Expect.expect("its row becomes installing at once", becomes?.note === "installing",
            becomes?.note);
        Expect.expect("and offers a second Return nothing to do", offers(becomes) === "",
            offers(becomes));
    }

    // The daemon serves Install as going: its step, where Joel pressed
    // Return, says so and offers nothing, so a second Return there only
    // closes, and the install was asked once.
    function going() {
        const { install, sure } = sureStep();
        if (sure?.note !== "installing")
            return;
        root.waiting = "the end";
        Expect.expect("Install says it's installing", install.note === "installing",
            install.note);
        Expect.expect("its step offers a Return nothing to do", offers(sure) === "",
            offers(sure));
        Expect.expect("the install asked once, of that device", root.installs.join() === "b10",
            root.installs.join());
        Qt.callLater(() => Qt.exit(0));
    }
}
