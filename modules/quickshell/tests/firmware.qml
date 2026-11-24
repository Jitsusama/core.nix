pragma ComponentBehavior: Bound
// Drives the launcher's Firmware (modules/quickshell/launcher/Firmware.qml)
// through Quickshell itself, over a stand-in for the machine daemon
// serving a device's Install ready (the ready-row form, as
// firmware_rows.rs makes it), with the launcher stood in, open, "firmware"
// typed. Install's step asking if Joel's sure asks the daemon's install
// once, and the row it becomes at once, then the step the daemon serves
// while it installs, offer nothing to run: a Return on either only closes
// (Launcher.qml's run), so a second Return never installs again, as the
// desktop test's dropped case held (#5869's F250 and F261). Its first
// rows are the watch's whole answer, which come before the link counts it
// answered, so they're seen only because Firmware answers again once it
// does (Firmware.qml's onAnsweredChanged); without that the case fails as
// never answered again. It exits 0 once every case holds, and otherwise
// 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "Expect.js" as Expect
import "Watch.js" as Watch
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: root.firstRows
    readonly property string firstRows: "the first rows, answered again once the watch is"
        + " answered whole"

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
                if (root.waiting === root.firstRows)
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

    // What a Return would do with a row, by some of Launcher.qml's run's
    // branches, in run's order: fill the field with its start, do its
    // Return's act, step into its category or group, ask its words, step
    // into the verbs of one that can't be taken back, step into what one
    // with no run of its own holds, close where it has nothing to do, open
    // its collection where it has no run, or run. It isn't run: it leaves
    // out a gone row, one held still, a step into a target or a copy's
    // contents, that a confirm inside a held step runs unless collecting,
    // that a row with a below of its own, run from below another, closes
    // and opens that below in place of its run, and how a row that stays,
    // opens or acts now runs. Nor is it what Return weighs before run: a
    // Return held under a guessed step (Held.returnUnder), then Arming.js's
    // returnAct, which gives words being asked for, does nothing on a
    // Return doubling a landing, and arms, steps into a thing, lines an end
    // or lands. So it tells this check's rows apart, and stands for run on
    // no other.
    function offers(row) {
        const holds = (row?.verbs?.() ?? []).length > 0 || !!row?.targets;
        if (row?.start !== undefined)
            return "fills";
        if (row?.returns)
            return "returns";
        if ((row?.category || row?.group) && holds)
            return "steps in";
        if (row?.given)
            return "asks";
        if ((row?.confirm || typeof row?.run !== "function") && holds)
            return "steps in";
        if (typeof row?.run !== "function")
            return row?.opens ? "opens" : "closes";
        return "runs";
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
        Expect.expect("the sure step's Return does its act", offers(sure) === "runs",
            offers(sure));
        Expect.expect("and its row says it's sure, a field Launcher.lands reads",
            sure.sure === true, sure.sure);
        theLauncher.answer = theFirmware.answer("firmware");
        sure.run();
        const becomes = theLauncher.becoming["firmware:b10:install:sure"];
        Expect.expect("its row becomes installing at once", becomes?.note === "installing",
            becomes?.note);
        Expect.expect("and a second Return there only closes", offers(becomes) === "closes",
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
        Expect.expect("its step's Return only closes", offers(sure) === "closes",
            offers(sure));
        Expect.expect("the install asked once, of that device", root.installs.join() === "b10",
            root.installs.join());
        Qt.callLater(() => Qt.exit(0));
    }
}
