// Where Reopen Them All puts each session back, run in Qt's own JavaScript
// engine, the one the shell runs it in, and how fast it may go. The
// `reopening-finds-each-screen` check copies modules/quickshell/Places.js,
// Reopening.js and Viewers.js beside this file and runs
// `qml -platform offscreen places.qml`. Each case is one a boot or a
// dock can make: the same monitor on another connector, a screen gone, and
// a place kept before screens were named by what they are. Long because
// every case runs in the one engine the check starts, so splitting it
// would start that engine again for each part, and its cases share the
// screens and places they're built from.
import QtQuick
import "Places.js" as Places
import "Reopening.js" as Reopening
import "Viewers.js" as Viewers

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

    function run() {
        const failures = [];
        const expect = (what, got, wanted) => {
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };
        const monitor = "HP Inc. HP Z27k G3 CN41282GC1";
        const panel = "LG Display 0x07C6";

        expect("a screen as niri's config names it", Places.screensOf({
            "DP-4": { make: "HP Inc.", model: "HP Z27k G3", serial: "CN41282GC1" },
            "eDP-1": { make: "LG Display", model: "0x07C6", serial: null }
        }), { "DP-4": monitor, "eDP-1": panel });
        expect("a screen that's off as gone, as the lid shut on the dock leaves it",
            Places.screensOf({
                "DP-4": { make: "HP Inc.", model: "HP Z27k G3", serial: "CN41282GC1",
                    logical: { x: 0, y: 0, width: 2560, height: 1440 } },
                "eDP-1": { make: "LG Display", model: "0x07C6", serial: null, logical: null }
            }), { "DP-4": monitor });

        // Kept on the monitor as DP-4; after the boot it's DP-5, beside the
        // laptop's panel, which is in use.
        const kept = [
            { screen: monitor, output: "DP-4", workspace: 2, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 1, column: 3, tile: 2 },
            null
        ];
        expect("each on its own screen, under its new connector",
            Places.placed(kept, { "DP-5": monitor, "eDP-1": panel }, "eDP-1"), [
                { screen: monitor, output: "DP-5", workspace: 2, column: 1, tile: 1 },
                { screen: panel, output: "eDP-1", workspace: 1, column: 3, tile: 2 },
                null
            ]);

        // Docked with the lid shut: the panel's sessions go on the
        // monitor, after its own workspaces, in their order.
        const docked = [
            { screen: monitor, output: "DP-4", workspace: 1, column: 1, tile: 1 },
            { screen: monitor, output: "DP-4", workspace: 3, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 2, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 1, column: 2, tile: 1 }
        ];
        expect("a gone screen's workspaces after the one in use's",
            Places.placed(docked, { "DP-4": monitor }, "DP-4").map(at => [at.output, at.workspace]),
            [["DP-4", 1], ["DP-4", 3], ["DP-4", 5], ["DP-4", 4]]);

        // The monitor is another one now, on the same connector: its
        // description says it isn't the screen the places were kept on.
        expect("a connector now another screen's isn't taken for it",
            Places.placed([docked[0]], { "DP-4": "Dell Inc. DELL U2723QE 7XY" }, "DP-4")
                .map(at => [at.output, at.workspace]), [["DP-4", 1]]);
        expect("two gone screens, one's workspaces after the other's",
            Places.placed([docked[0], docked[2]], { "DP-4": "Dell Inc. DELL U2723QE 7XY" }, "DP-4")
                .map(at => at.workspace), [1, 3]);

        // Two monitors alike, with no serial to tell them apart: each
        // place goes to the one it was kept on.
        const twin = "QEMU QEMU Monitor";
        expect("two screens alike, each place on its own",
            Places.placed([{ screen: twin, output: "Virtual-2", workspace: 1, column: 1, tile: 1 }],
                { "Virtual-1": twin, "Virtual-2": twin }, "Virtual-1").map(at => at.output),
            ["Virtual-2"]);

        // Kept before screens were named so: found by its connector.
        const older = [{ output: "eDP-1", workspace: 2, column: 1, tile: 1 }];
        expect("a place without a screen, by its connector",
            Places.placed(older, { "eDP-1": panel, "DP-4": monitor }, "DP-4")
                .map(at => [at.output, at.workspace]), [["eDP-1", 2]]);
        // Only a connector niri said is one: a prototype's name never is, so
        // a place kept on one is gone, and noted away from it (noteAway).
        expect("a place kept on a prototype's name is gone, to the screen in use",
            Places.placed([{ output: "constructor", workspace: 2, column: 1, tile: 1 }],
                { "eDP-1": panel }, "eDP-1").map(at => [at.output, at.away?.screen ?? null]),
            [["eDP-1", "constructor"]]);
        // Nor heard on one: a twin kept on a prototype's name is among the
        // others a gone twin's note says, as any twin gone is.
        const protoTwin = { screen: twin, output: "Virtual-1", workspace: 1, column: 1, tile: 1 };
        expect("a twin kept on a prototype's name is among the others it left",
            Places.whereKept(protoTwin, [protoTwin, Object.assign({}, protoTwin,
                { output: "toString" })], { "eDP-1": panel }, { inUse: "eDP-1" }).away.others,
            { "eDP-1": panel, "toString": twin });

        // No screen known, as when niri didn't answer: left as it was kept,
        // rather than every place taken for a gone screen's.
        expect("no screen known leaves it as kept",
            Places.placed([docked[2]], {}, "DP-4").map(at => [at.output, at.workspace]),
            [["eDP-1", 2]]);
        expect("no screen in use leaves a gone one as kept",
            Places.placed([docked[2]], { "DP-4": monitor }, null)
                .map(at => [at.output, at.workspace]), [["eDP-1", 2]]);

        // A gone screen's place says where it belongs, so its workspace
        // can go there when the screen comes; one on its screen doesn't.
        // It also says which connectors were other screens as it was found
        // gone, since a reopen notes it minutes later, by when the screen
        // may have come back on one of them.
        expect("a gone screen's place says where it belongs",
            Places.placed(docked, { "DP-4": monitor }, "DP-4").map(at => at.away ?? null), [
                null, null,
                { screen: panel, workspace: 2, output: "eDP-1", others: { "DP-4": monitor } },
                { screen: panel, workspace: 1, output: "eDP-1", others: { "DP-4": monitor } }
            ]);
        // Two alike screens are two only when one moment, the other
        // windows a boot kept at once, shows windows on both.
        const twins = [{ screen: twin, output: "Virtual-1", workspace: 1, column: 1, tile: 1 },
            { screen: twin, output: "Virtual-2", workspace: 1, column: 1, tile: 1 }];
        expect("two alike screens gone at a boot are other screens to each other",
            Places.placed(twins, { "eDP-1": panel }, "eDP-1", [twins, twins]).map(at => at.away.others), [
                { "eDP-1": panel, "Virtual-2": twin },
                { "eDP-1": panel, "Virtual-1": twin }
            ]);
        // One alike twin of the two kept back: the place kept on it stays,
        // and the other's screen is gone, rather than both taken for it.
        expect("a twin kept beside another isn't taken for it",
            Places.placed(twins, { "Virtual-1": twin }, "Virtual-1", [twins, twins])
                .map(at => [at.output, at.workspace, at.away?.others ?? null]),
            [["Virtual-1", 1, null], ["Virtual-1", 2, { "Virtual-1": twin }]]);
        // Both renumbered, as a dock does: each takes a connector of its
        // own, and with only one back the other's is gone.
        expect("twins back renumbered share their connectors out",
            Places.placed(twins, { "Virtual-3": twin, "Virtual-4": twin }, "Virtual-3", [twins, twins])
                .map(at => at.output), ["Virtual-3", "Virtual-4"]);
        // Windows put back one at a time do the same, each told those given.
        const bothBack = { "Virtual-3": twin, "Virtual-4": twin };
        const firstBack = Places.whereKept(twins[0], twins, bothBack, { inUse: "Virtual-3" });
        expect("twin windows back renumbered share their connectors out",
            [firstBack.output, Places.whereKept(twins[1], twins, bothBack,
                { given: [{ at: twins[0], output: firstBack.output }], inUse: "Virtual-3" })
                .output], ["Virtual-3", "Virtual-4"]);
        // And go where their sessions went, whatever order each comes back in.
        const twinsLast = [twins[1], twins[0]];
        const sessions = Reopening.placedGiven(twinsLast, bothBack, "Virtual-3", [twins, twins],
            []);
        const sessionsGiven = sessions.given;
        expect("a twin window back renumbered goes where its sessions went",
            [Places.whereKept(twins[0], twins, bothBack,
                { given: sessionsGiven, inUse: "Virtual-3" }).output,
                sessionsGiven.map(one => one.output), sessions.places.map(at => at.output)],
            ["Virtual-4", ["Virtual-3", "Virtual-4"], ["Virtual-3", "Virtual-4"]]);
        // A window back before its sessions were placed gives way to them,
        // so the other twin's windows still have a connector to go to; one
        // whose twin's sessions said nothing keeps what it took.
        const early = [{ at: twins[0], output: "Virtual-3" }];
        const seeded = Reopening.placedGiven(twinsLast, bothBack, "Virtual-3", [twins, twins],
            early).given;
        expect("a window back before its sessions gives way to them",
            [seeded.map(one => one.output), Places.whereKept(twins[1], twins, bothBack,
                { given: seeded, inUse: "Virtual-3" }).output],
            [["Virtual-3", "Virtual-4"], "Virtual-3"]);
        const twinFirst = Reopening.placedGiven([twins[1]], bothBack, "Virtual-3", [twins], early)
            .given;
        expect("nor keeps a connector the other twin's sessions were given",
            [twinFirst.map(one => one.output), Places.whereKept(twins[1], twins, bothBack,
                { given: twinFirst, inUse: "Virtual-3" }).output], [["Virtual-3"], "Virtual-3"]);
        // Nor to another screen's session, though its connector is called
        // what the twin's was when kept.
        const panelThere = { "Virtual-1": panel, "Virtual-3": twin, "Virtual-4": twin };
        const panelAt = { screen: panel, output: "Virtual-1", workspace: 1, column: 1, tile: 1 };
        const panelFirst = Reopening.placedGiven([panelAt], panelThere, "Virtual-1", [[]], early)
            .given;
        expect("nor gives way to another screen's session",
            [panelFirst.map(one => one.output), Places.whereKept(twins[1], twins, panelThere,
                { given: panelFirst, inUse: "Virtual-1" }).output],
            [["Virtual-1", "Virtual-3"], "Virtual-4"]);
        // Nor to a session of its twin's given no connector: sent to another
        // screen in use, or left as kept, with no screen said or none in use,
        // though last boot's connector is another screen's now. (One kept
        // away on a screen showing its own is the twin gone's case below.)
        const tookFirst = [{ at: twins[1], output: "Virtual-3" }];
        const givenNone = (why, screens, inUse) => {
            const given = Reopening.placedGiven([twins[1]], screens, inUse, [twins], tookFirst)
                .given;
            expect(`nor gives way to its twin's session ${why}`,
                [given.map(one => one.output ?? "none"), Places.whereKept(twins[0], twins,
                    bothBack, { given, inUse: "Virtual-3" }).output],
                [["none", "Virtual-3"], "Virtual-4"]);
        };
        givenNone("sent to another screen in use", { "eDP-1": panel }, "eDP-1");
        givenNone("left as kept with no screen said", {}, "Virtual-3");
        givenNone("left as kept with none in use", { "eDP-1": panel }, null);
        givenNone("left as kept on another screen's connector", { "Virtual-2": panel }, null);
        // A window put back gives the connector it's found on only while
        // that shows its screen: a twin's, one kept before screens were
        // named, never last boot's with no screen said, nor a twin's on one
        // not heard yet, nor a prototype's name, nor any with no screens.
        const windowOn = (at, screens, inUse) =>
            Reopening.keptBack(at, twins, screens, [], inUse).gives.output ?? "none";
        const unnamed = { output: "Virtual-3", workspace: 1, column: 1, tile: 1 };
        expect("a window put back gives only a connector showing its screen",
            [windowOn(twins[1], bothBack, "Virtual-3"), windowOn(unnamed, bothBack, "Virtual-3"),
                windowOn(twins[1], {}, null), windowOn(twins[1], { "Virtual-2": "" }, "Virtual-2"),
                Reopening.givenOn("", { output: "constructor" }, {}) ?? "none",
                Reopening.givenOn(twin, { output: "Virtual-3" }) ?? "none"],
            ["Virtual-3", "Virtual-3", "none", "none", "none", "none"]);
        // It goes back by where it's found, but gives only givenOn's: found
        // on last boot's connector with no screen said, or on one not heard
        // yet, it goes back there and gives none; found on the connector its
        // own sessions were given, it gives that one.
        const backAt = (screens, given, inUse) => {
            const back = Reopening.keptBack(twins[1], twins, screens, given, inUse);
            return [back.kept.output, back.gives.at === twins[1], back.gives.output ?? "none"];
        };
        expect("a window put back gives givenOn's connector, not the one it's found on",
            [backAt({}, [], null), backAt({ "Virtual-2": "" }, [], "Virtual-2"),
                backAt(bothBack, sessionsGiven, "Virtual-3")],
            [["Virtual-2", true, "none"], ["Virtual-2", true, "none"],
                ["Virtual-3", true, "Virtual-3"]]);
        expect("one whose twin's sessions said nothing keeps what it took",
            Places.whereKept(twins[1], twins, bothBack,
                { given: Reopening.givenBy([], [], early, {}), inUse: "eDP-1" }).output,
            "Virtual-4");
        // A twin gone gives its windows no connector, so none is kept from
        // them, and one kept nowhere gives nothing.
        const oneTwin = [twins[0], twins[1], null];
        expect("a twin gone gives its windows no connector, nor one kept nowhere",
            Reopening.placedGiven(oneTwin, { "Virtual-3": twin }, "Virtual-3",
                [twins, twins, []], []).given.map(one => [one.at.output, one.output ?? "none"]),
            [["Virtual-1", "Virtual-3"], ["Virtual-2", "none"]]);
        // A window back joins its own twin's windows, not the other's, though
        // both monitors are one screen to look at.
        const twinBack = (id, output, workspace) => ({ id, at: { screen: twin, output, workspace,
            column: 1, tile: 1 }, workspace: { id: 30 + id, idx: workspace, output }, column: 1 });
        expect("a window back doesn't join its twin's workspace",
            Places.whereBack(Object.assign({}, twins[0], { column: 2 }),
                [twinBack(1, "Virtual-2", 1)], Places.twinsOf(twins[0], twins)),
            { make: true, output: null, index: 1 });
        expect("a window back joins its own twin's workspace",
            Places.whereBack(Object.assign({}, twins[0], { column: 2 }),
                [twinBack(1, "Virtual-2", 1), twinBack(2, "Virtual-1", 1)],
                Places.twinsOf(twins[0], twins)),
            { join: 32, after: 2 });
        expect("one twin back renumbered is the first's, and the other's is gone",
            Places.placed(twins, { "Virtual-3": twin }, "Virtual-3", [twins, twins])
                .map(at => [at.output, at.away?.others ?? null]),
            [["Virtual-3", null], ["Virtual-3", { "Virtual-3": twin, "Virtual-1": twin }]]);
        // One monitor kept on two connectors in one boot, as a session
        // ended before a redock keeps its old place, with no moment showing
        // it on both: one screen, so neither is the other's, gone or found.
        const renumbered = [{ screen: monitor, output: "DP-4", workspace: 1, column: 1, tile: 1 },
            { screen: monitor, output: "DP-5", workspace: 2, column: 1, tile: 1 }];
        const moment = [renumbered[1]];
        const afterRedock = [moment, moment];
        expect("one monitor kept on two connectors isn't another screen to itself",
            Places.placed(renumbered, { "eDP-1": panel }, "eDP-1", afterRedock)
                .map(at => at.away.others), [{ "eDP-1": panel }, { "eDP-1": panel }]);
        expect("one monitor kept on two connectors is found on its one",
            Places.placed(renumbered, { "DP-5": monitor, "eDP-1": panel }, "eDP-1", afterRedock)
                .map(at => at.output), ["DP-5", "DP-5"]);

        // niri keeps a workspace on each screen it has connected, an empty
        // one where Joel has none, and homeward counts a screen there only
        // so. These cases name the screens connected, so each is given the
        // empty one niri would list; the stale-screen cases below call
        // Places.homeward bare, listing niri's workspaces themselves.
        function homeward(away, screens, workspaces) {
            const empty = Object.keys(screens).map((output, at) => ({ id: 900 + at, output }));
            return Places.homeward(away, screens, workspaces.concat(empty));
        }
        // The monitor comes after the reopen began: the workspaces given
        // the panel in its place go to it, lowest kept number first, their
        // order there the windows daemon's; one gone stays gone, and one
        // there already is marked and left.
        const away = { [monitor]: [
            { id: 11, workspace: 1 }, { id: 13, workspace: 3 }, { id: 12, workspace: 2 },
            { id: 14, workspace: 4 }, { id: 15, workspace: 5 }
        ] };
        const workspaces = [
            { id: 11, output: "eDP-1" }, { id: 12, output: "eDP-1" }, { id: 13, output: "eDP-1" },
            { id: 15, output: "DP-4" }, { id: 16, output: "DP-4" }
        ];
        expect("a screen come back takes its workspaces, lowest kept first",
            homeward(away, { "eDP-1": panel, "DP-4": monitor }, workspaces), {
                moves: [{ output: "DP-4", id: 11 },
                    { output: "DP-4", id: 12 },
                    { output: "DP-4", id: 13 },
                    { output: "DP-4", id: 15, there: true }],
                waiting: {}
            });
        expect("a screen still gone keeps its workspaces waiting, but not one gone",
            homeward(away, { "eDP-1": panel }, workspaces), { moves: [], waiting: {
                [monitor]: [{ id: 11, workspace: 1 }, { id: 12, workspace: 2 },
                    { id: 13, workspace: 3 }, { id: 15, workspace: 5 }]
            } });
        // The monitor is on another connector now, which was no other
        // screen as it left, or another screen's then, so this is it.
        const another = { [monitor]: [{ id: 11, workspace: 1, output: "DP-4", others: { "eDP-1": panel } }] };
        expect("a screen come back on another connector takes its workspaces",
            homeward(another, { "eDP-1": panel, "DP-5": monitor }, [{ id: 11, output: "eDP-1" }]),
            { moves: [{ output: "DP-5", id: 11 }], waiting: {} });
        expect("a screen come back on a connector another's as it left takes its workspaces",
            homeward({ [monitor]: [{ id: 11, workspace: 1, output: "DP-4",
                others: { "DP-5": "Dell Inc. DELL U2723QE 7XY" } }] },
                { "eDP-1": panel, "DP-5": monitor }, [{ id: 11, output: "eDP-1" }]),
            { moves: [{ output: "DP-5", id: 11 }], waiting: {} });
        // One monitor noted twice, from two connectors at two times (a
        // reopen's kept place, then an unplug): both are it, so both go.
        const twice = { [monitor]: [{ id: 1, workspace: 1, output: "DP-4", others: {} },
            { id: 2, workspace: 2, output: "DP-5", others: { "eDP-1": panel } }] };
        expect("one monitor noted from two connectors takes both workspaces",
            homeward(twice, { "eDP-1": panel, "DP-5": monitor },
                [{ id: 1, output: "eDP-1" }, { id: 2, output: "eDP-1" }]), {
                moves: [{ output: "DP-5", id: 1 }, { output: "DP-5", id: 2 }], waiting: {}
            });

        // Two screens alike, as two monitors of one model without serials
        // are, and as a VM's are. Each workspace decides by the connector it
        // left: it goes back there, however niri lists them, rather than to
        // an alike screen as if home (vm-screens-keep-places flaked so,
        // 2026-11-16 and 18 Nov).
        const qemu = "Red Hat, Inc. QEMU Monitor Unknown";
        const left = { [qemu]: [{ id: 21, workspace: 2, output: "Virtual-2",
            others: { "eDP-1": panel, "Virtual-1": qemu } }] };
        const onThePanel = [{ id: 21, output: "eDP-1" }];
        expect("a workspace goes back to the alike screen it left, wherever it sits",
            homeward(left, { "eDP-1": panel, "Virtual-1": qemu, "Virtual-2": qemu },
                [{ id: 21, output: "Virtual-1" }]),
            { moves: [{ output: "Virtual-2", id: 21 }], waiting: {} });
        expect("a workspace niri handed back to the screen it left is there",
            homeward(left, { "Virtual-1": qemu, "Virtual-2": qemu }, [{ id: 21, output: "Virtual-2" }]),
            { moves: [{ output: "Virtual-2", id: 21, there: true }], waiting: {} });
        // While the screen it left is gone, an alike screen that was
        // connected when it was noted isn't that screen come back, so it
        // waits wherever it is: on the alike screen it was sent to, on
        // another (as Joel might move it), or beside two more alike. Taken
        // for home, its note was dropped and the replug left it where it was.
        const waits = { moves: [], waiting: left };
        expect("a workspace on the alike screen it was sent to waits for the one it left",
            homeward(left, { "Virtual-1": qemu }, [{ id: 21, output: "Virtual-1" }]), waits);
        expect("a workspace off the alike screen still there waits for the one it left",
            homeward(left, { "eDP-1": panel, "Virtual-1": qemu }, onThePanel), waits);
        // niri's screens are asked for apart from its workspaces, and under
        // load the answer can be from before the screen it left went, so it
        // still lists that connector. A screen is there only while niri lists
        // a workspace on it, as it does on each it has connected; taken for
        // there, the workspace was sent to a gone connector and its note
        // dropped (vm-screens-keep-places, 21 and 22 Nov).
        expect("a screen niri said before it went isn't there to go back to",
            Places.homeward(left, { "Virtual-1": qemu, "Virtual-2": qemu },
                [{ id: 21, output: "Virtual-1" }]), waits);
        expect("an alike screen niri said before it went isn't one new since",
            Places.homeward(left, { "Virtual-1": qemu, "Virtual-3": qemu },
                [{ id: 21, output: "Virtual-1" }]), waits);
        // The other way round, the screens heard before niri says the
        // workspace it makes on one come, it waits as just above, and goes
        // once that's said.
        expect("a screen come back goes once niri lists a workspace on it",
            Places.homeward(left, { "Virtual-1": qemu, "Virtual-2": qemu },
                [{ id: 21, output: "Virtual-1" }, { id: 22, output: "Virtual-2" }]),
            { moves: [{ output: "Virtual-2", id: 21 }], waiting: {} });
        expect("the screens connected are those niri lists a workspace on",
            Places.connected({ "Virtual-1": qemu, "Virtual-2": qemu, "eDP-1": panel },
                [{ id: 21, output: "Virtual-1" }, { id: 22, output: "eDP-1" }]),
            { "Virtual-1": qemu, "eDP-1": panel });
        // Before niri lists any workspace, as when Reopen Them All comes
        // first, nothing says a screen went, and each said keeps its
        // connector found anew, as before the rule.
        expect("with no workspace listed yet each screen said is taken for there",
            Places.connected({ "Virtual-1": qemu, "eDP-1": panel }, []),
            { "Virtual-1": qemu, "eDP-1": panel });
        // A screen niri lists a workspace on that the answer to Outputs
        // predates is there all the same, undescribed (""). Left out, as
        // when that answer named only the monitor since unplugged, the
        // screen in use was taken for none: Reopen Them All left the
        // monitor's places on its gone connector, noted nowhere, and took
        // the panel's own for a gone screen's.
        const onlyThePanel = [{ id: 21, output: "eDP-1" }];
        expect("a screen niri lists a workspace on but wasn't heard yet is there, undescribed",
            Places.connected({ "DP-4": monitor }, onlyThePanel), { "eDP-1": "" });
        expect("a workspace on no screen names none",
            Places.connected({ "DP-4": monitor }, [{ id: 20, output: null }].concat(onlyThePanel)),
            { "eDP-1": "" });
        // With no screen said at all, as when niri never answered and
        // Reopen Them All went ahead after screensPatience, nothing says
        // which screen is which: each place is left as it was kept, as
        // before, rather than every one off the screen in use taken for
        // a gone screen's.
        const bothListed = [{ id: 21, output: "DP-4" }, { id: 22, output: "eDP-1" }];
        expect("with no screen said, those listed aren't taken for unheard ones",
            Places.connected({}, bothListed), {});
        expect("with no screen said, each place is left as kept",
            Places.placed(docked, Places.connected({}, bothListed), "DP-4")
                .map(at => [at.output, at.workspace, at.away ?? null]),
            [["DP-4", 1, null], ["DP-4", 3, null], ["eDP-1", 2, null], ["eDP-1", 1, null]]);
        // A window put back so is left as kept too, and noted nowhere,
        // rather than sent to the screen in use beside sessions that stay.
        expect("with no screen said, a window put back is found where it was kept, not noted",
            Places.whereKept(docked[0], [], {}, { inUse: "eDP-1" }),
            { output: "DP-4", away: null });
        // Only the screen in use is taken for its own connector's while
        // it isn't heard: it holds a gone screen's places anyway, so no
        // place lands on a screen it wouldn't have. It may be another
        // screen now, so each place kept on it is noted too, and goes
        // home if its own turns up elsewhere. Another not heard may be
        // another screen as well, so a place kept on it is gone.
        const panelSoon = Places.connected({ "DP-4": monitor }, onlyThePanel);
        expect("a place on the screen in use not heard yet stays, noted, a gone one's after",
            Places.placed(docked, panelSoon, "eDP-1")
                .map(at => [at.output, at.workspace, at.away?.others ?? null]),
            [["eDP-1", 3, {}], ["eDP-1", 5, {}], ["eDP-1", 2, {}], ["eDP-1", 1, {}]]);
        // Each answer below is stale: it names the monitor as it was,
        // on a connector niri lists no workspace on now.
        const panelAndFour = onlyThePanel.concat([{ id: 22, output: "DP-4" }]);
        const fourSoon = Places.connected({ "DP-3": monitor }, panelAndFour);
        expect("a place on a screen not heard yet, not in use, is gone",
            Places.placed([docked[0]], fourSoon, "eDP-1")
                .map(at => [at.output, at.away?.screen ?? null]), [["eDP-1", monitor]]);
        expect("a place kept on the screen in use not heard yet is found there",
            Places.whereKept(docked[2], [], panelSoon, { inUse: "eDP-1" }).output, "eDP-1");
        // Heard as another screen, the one in use isn't the place's own.
        expect("a place kept on the screen in use heard as another is gone",
            Places.whereKept(docked[2], [], { "eDP-1": monitor }, { inUse: "eDP-1" }).output,
            undefined);
        // Its own screen heard on another connector is where it goes,
        // as a monitor renumbered while its old connector is now the
        // screen in use, not heard yet.
        expect("a place whose screen is heard elsewhere goes there, not to the one in use",
            Places.placed([docked[2]], { "eDP-1": "", "DP-6": panel }, "eDP-1")
                .map(at => [at.output, at.workspace, at.away ?? null]), [["DP-6", 2, null]]);
        // A place kept by its connector alone is found by it, which is all
        // it has, so it isn't noted for its screen not being heard.
        expect("a place kept by its connector on the screen in use not heard yet isn't noted",
            Places.placed(older, { "eDP-1": "" }, "eDP-1").map(at => at.away ?? null), [null]);
        expect("a window put back by its connector on a screen not heard yet isn't noted",
            Places.awayOf(older[0], "eDP-1", [], { "eDP-1": "" }), null);
        // Nor is it gone from a screen not heard that isn't in use, as one
        // kept by what it is would be: its connector is all there is.
        expect("a place kept by its connector on a screen not heard, not in use, stays",
            Places.placed([{ output: "DP-5", workspace: 2, column: 1, tile: 1 }],
                { "eDP-1": panel, "DP-5": "" }, "eDP-1")
                .map(at => [at.output, at.workspace, at.away ?? null]), [["DP-5", 2, null]]);
        // A screen heard as nothing, no make, model or serial, can't be
        // told from one not heard, so it isn't another screen to a place
        // gone either: it may be its own, its description not read yet.
        expect("a screen heard as nothing isn't another screen to a place gone",
            Places.placed([docked[0]], { "eDP-1": panel, "DP-6": "" }, "eDP-1")[0].away?.others,
            { "eDP-1": panel });
        expect("a window put back on a screen not heard, not in use, is gone",
            Places.whereKept(docked[0], [], fourSoon, { inUse: "eDP-1" }).output, undefined);
        expect("a window put back on a screen gone carries the note its session does",
            [Places.whereKept(docked[0], [], fourSoon, { inUse: "eDP-1" }).away,
                Places.placed([docked[0]], fourSoon, "eDP-1")[0].away],
            [0, 1].map(() => ({ screen: monitor, workspace: 1, output: "DP-4", others: {} })));
        // The note a window put back so carries, the same as placed's.
        const panelNote = Places.whereKept(docked[2], [], panelSoon, { inUse: "eDP-1" }).away;
        expect("a window put back on the screen in use not heard yet is noted",
            panelNote, { screen: panel, workspace: 2, output: "eDP-1", others: {} });
        expect("a window put back on its screen heard isn't noted",
            Places.awayOf(docked[2], "eDP-1", [], { "eDP-1": panel }), null);
        // Heard as itself, the note is done; heard as another screen, as
        // a monitor kept on that connector before it was the panel's, or
        // one niri gives no make, model or serial, the note waits for its
        // own screen, which takes its places home wherever it comes.
        const panelWaits = { [panel]: [Object.assign({ id: 21 }, panelNote)] };
        expect("a place noted on the screen in use is home once that's heard as its own",
            Places.homeward(panelWaits, { "eDP-1": panel }, onlyThePanel),
            { moves: [{ output: "eDP-1", id: 21, there: true }], waiting: {} });
        expect("a place noted on the screen in use waits while that's not heard",
            Places.homeward(panelWaits, { "eDP-1": "" }, onlyThePanel),
            { moves: [], waiting: panelWaits });
        expect("a place noted on the screen in use waits once that's heard as another",
            Places.homeward(panelWaits, { "eDP-1": monitor }, onlyThePanel),
            { moves: [], waiting: panelWaits });
        expect("a place noted on the screen in use goes to its own once heard elsewhere",
            Places.homeward(panelWaits, { "eDP-1": monitor, "DP-6": panel },
                onlyThePanel.concat([{ id: 23, output: "DP-6" }])),
            { moves: [{ output: "DP-6", id: 21 }], waiting: {} });
        // A place kept before screens were named by what they are is
        // noted by its connector, which is all it has to know it by, so
        // it goes home as soon as niri lists a workspace there.
        expect("a place noted by its connector goes home once that's listed, heard or not",
            Places.homeward({ "DP-5": [{ id: 21, workspace: 1, output: "DP-5", others: {} }] },
                { "eDP-1": panel }, [{ id: 21, output: "eDP-1" }, { id: 22, output: "DP-5" }]),
            { moves: [{ output: "DP-5", id: 21 }], waiting: {} });
        expect("a place noted by its connector goes home once that's heard as any screen",
            Places.homeward({ "DP-5": [{ id: 21, workspace: 1, output: "DP-5", others: {} }] },
                { "eDP-1": panel, "DP-5": monitor },
                [{ id: 21, output: "eDP-1" }, { id: 22, output: "DP-5" }]),
            { moves: [{ output: "DP-5", id: 21 }], waiting: {} });
        // A note for no screen, as for a workspace first seen on none, goes
        // to no screen not heard yet, any of which it would match.
        const nowhere = { "": [{ id: 21, workspace: 1, output: "", others: {} }] };
        expect("a note for no screen waits rather than take a screen not heard",
            Places.homeward(nowhere, { "eDP-1": panel },
                [{ id: 21, output: "eDP-1" }, { id: 22, output: "DP-5" }]),
            { moves: [], waiting: nowhere });
        // A screen not heard as a place is found gone may be its own,
        // renumbered, so it isn't noted as another screen: noted so, the
        // monitor back on DP-5 after a boot never took its workspace home.
        const panelAndFive = onlyThePanel.concat([{ id: 22, output: "DP-5" }]);
        const renumberedSoon = Places.placed([docked[0]],
            Places.connected({ "DP-4": monitor }, panelAndFive),
            "eDP-1")[0].away ?? null;
        expect("a screen not heard as one is found gone isn't another screen to it",
            renumberedSoon?.others ?? null, {});
        expect("a screen renumbered, not heard as it was found gone, takes it home once heard",
            Places.homeward({ [monitor]: [Object.assign({ id: 21 }, renumberedSoon)] },
                { "eDP-1": panel, "DP-5": monitor }, panelAndFive),
            { moves: [{ output: "DP-5", id: 21 }], waiting: {} });
        // A twin in use not heard yet keeps its place, and is another
        // screen to its twin gone, which doesn't take it once it's heard.
        const onTheFirst = [{ id: 21, output: "Virtual-1" }];
        const twinsSoon = Places.placed(twins, Places.connected({ "eDP-1": panel }, onTheFirst),
            "Virtual-1", [twins, twins]);
        expect("a twin in use not heard yet keeps its place, another screen to its twin gone",
            twinsSoon.map(at => [at.output, at.workspace, at.away?.others ?? null]),
            [["Virtual-1", 1, { "Virtual-2": twin }], ["Virtual-1", 2, { "Virtual-1": twin }]]);
        const twinWaits = { [twin]: [Object.assign({ id: 21 }, twinsSoon[1].away ?? null)] };
        expect("a twin gone doesn't take its twin once that's heard",
            Places.homeward(twinWaits, { "Virtual-1": twin }, onTheFirst),
            { moves: [], waiting: twinWaits });
        // The shell keeps both twins' notes under their one screen.
        const bothWait = { [twin]: [Object.assign({ id: 20 }, twinsSoon[0].away ?? null),
            Object.assign({ id: 21 }, twinsSoon[1].away ?? null)] };
        const bothOnTheFirst = [{ id: 20, output: "Virtual-1" }].concat(onTheFirst);
        expect("twins noted together: the one in use is home, its twin gone waits",
            Places.homeward(bothWait, { "Virtual-1": twin }, bothOnTheFirst),
            { moves: [{ output: "Virtual-1", id: 20, there: true }],
                waiting: { [twin]: [bothWait[twin][1]] } });
        expect("twins noted together: the twin gone goes once it's heard",
            Places.homeward(bothWait, { "Virtual-1": twin, "Virtual-2": twin },
                bothOnTheFirst.concat([{ id: 22, output: "Virtual-2" }])),
            { moves: [{ output: "Virtual-1", id: 20, there: true },
                { output: "Virtual-2", id: 21 }], waiting: {} });
        // Its twin's note alone keeps one off the twin's connector, as when
        // its own was made before that connector was heard as its twin.
        const byTwin = { [twin]: [Object.assign({ id: 20 }, twinsSoon[0].away ?? null),
            { id: 21, workspace: 1, output: "Virtual-2", others: {} }] };
        expect("a twin's note alone keeps the one gone off its connector",
            Places.homeward(byTwin, { "Virtual-1": twin }, bothOnTheFirst),
            { moves: [{ output: "Virtual-1", id: 20, there: true }],
                waiting: { [twin]: [Object.assign({}, byTwin[twin][1],
                    { others: { "Virtual-1": twin } })] } });
        const three = { [qemu]: [{ id: 21, workspace: 2, output: "Virtual-2",
            others: { "Virtual-1": qemu, "Virtual-3": qemu } }] };
        expect("a third alike screen isn't the one it left",
            homeward(three, { "Virtual-1": qemu, "Virtual-3": qemu },
                [{ id: 21, output: "Virtual-1" }]), { moves: [], waiting: three });
        // A connector connected as it left whose screen wasn't heard yet
        // may have been an alike one, just plugged in, so it isn't taken.
        const unheard = { [qemu]: [{ id: 21, workspace: 2, output: "Virtual-2", others: { "Virtual-1": "" } }] };
        expect("a connector not heard as it left isn't the one it left",
            homeward(unheard, { "Virtual-1": qemu }, [{ id: 21, output: "Virtual-1" }]),
            { moves: [], waiting: unheard });
        // The screen it left can come back alike on a connector that was
        // no other screen as it left, as a dock that numbers them anew does.
        expect("a workspace goes to an alike connector new since it left",
            homeward(left, { "Virtual-1": qemu, "Virtual-3": qemu }, [{ id: 21, output: "Virtual-1" }]),
            { moves: [{ output: "Virtual-3", id: 21 }], waiting: {} });
        // Both alike screens gone at once, as at an undock: each workspace
        // goes to the one it left, one there already left there, and one
        // whose screen is still gone doesn't take its sibling's for it.
        const both = { [qemu]: [{ id: 1, workspace: 1, output: "Virtual-1", others: { "Virtual-2": qemu } },
            { id: 2, workspace: 1, output: "Virtual-2", others: { "Virtual-1": qemu } }] };
        expect("both alike screens back, each takes its own",
            homeward(both, { "Virtual-1": qemu, "Virtual-2": qemu },
                [{ id: 1, output: "Virtual-1" }, { id: 2, output: "Virtual-1" }]), {
                moves: [{ output: "Virtual-1", id: 1, there: true }, { output: "Virtual-2", id: 2 }],
                waiting: {}
            });
        // As comeHome runs it, again on each change: what still waits is
        // fed back, and goes once the screen it left comes.
        const oneBack = [{ id: 1, output: "Virtual-2" }, { id: 2, output: "Virtual-2" }];
        const first = homeward(both, { "Virtual-2": qemu }, oneBack);
        expect("one alike screen back, the other's workspace waits beside it", first, {
            moves: [{ output: "Virtual-2", id: 2, there: true }],
            waiting: { [qemu]: [both[qemu][0]] }
        });
        expect("what waits fed back keeps waiting while its screen is gone",
            homeward(first.waiting, { "Virtual-2": qemu }, oneBack),
            { moves: [], waiting: first.waiting });
        expect("what waits fed back goes once its screen comes",
            homeward(first.waiting, { "Virtual-1": qemu, "Virtual-2": qemu }, oneBack),
            { moves: [{ output: "Virtual-1", id: 1 }], waiting: {} });
        // Back alike on two new connectors, as a dock numbering them anew:
        // each takes its own, rather than both the first.
        expect("two alike screens back on new connectors share them out",
            homeward(both, { "eDP-1": panel, "DP-6": qemu, "DP-7": qemu },
                [{ id: 1, output: "eDP-1" }, { id: 2, output: "eDP-1" }]), {
                moves: [{ output: "DP-6", id: 1 }, { output: "DP-7", id: 2 }], waiting: {}
            });
        // Back one by one: the one still waiting keeps the first's new
        // connector as another screen's, so the next call doesn't take it.
        const offThePanel = [{ id: 1, output: "eDP-1" }, { id: 2, output: "eDP-1" }];
        const oneNew = homeward(both, { "eDP-1": panel, "DP-6": qemu }, offThePanel);
        expect("one alike screen back on a new connector, the other waits knowing it", oneNew, {
            moves: [{ output: "DP-6", id: 1 }],
            waiting: { [qemu]: [Object.assign({}, both[qemu][1],
                { others: { "Virtual-1": qemu, "DP-6": qemu } })] }
        });
        expect("the other alike screen back on a new connector, its workspace takes that",
            homeward(oneNew.waiting, { "eDP-1": panel, "DP-6": qemu, "DP-7": qemu }, offThePanel),
            { moves: [{ output: "DP-7", id: 2 }], waiting: {} });
        // Either was another screen as the other left, as when one went
        // first: the second leaving's note names the first, not the other
        // way, and still neither takes the other's.
        const oneWay = { [qemu]: [{ id: 1, workspace: 1, output: "Virtual-1", others: { "Virtual-2": qemu } },
            { id: 2, workspace: 2, output: "Virtual-2", others: {} }] };
        expect("a sibling noted one way still doesn't take the other's",
            homeward(oneWay, { "DP-6": qemu }, offThePanel), {
                moves: [{ output: "DP-6", id: 1 }],
                waiting: { [qemu]: [Object.assign({}, oneWay[qemu][1],
                    { others: { "DP-6": qemu, "Virtual-1": qemu } })] }
            });
        const otherWay = { [qemu]: [{ id: 1, workspace: 1, output: "Virtual-1", others: {} },
            { id: 2, workspace: 2, output: "Virtual-2", others: { "Virtual-1": qemu } }] };
        expect("a sibling noted the other way still doesn't take the other's",
            homeward(otherWay, { "DP-6": qemu }, offThePanel), {
                moves: [{ output: "DP-6", id: 1 }],
                waiting: { [qemu]: [Object.assign({}, otherWay[qemu][1],
                    { others: { "Virtual-1": qemu, "DP-6": qemu } })] }
            });
        // The one that wasn't noted knowing its twin goes first, and its
        // twin's own screen comes back: it isn't taken for its own.
        const unnotedFirst = { [qemu]: [{ id: 1, workspace: 2, output: "Virtual-1", others: { "Virtual-2": qemu } },
            { id: 2, workspace: 1, output: "Virtual-2", others: {} }] };
        expect("a twin ranked first doesn't take its twin's own screen",
            homeward(unnotedFirst, { "Virtual-1": qemu }, offThePanel), {
                moves: [{ output: "Virtual-1", id: 1 }],
                waiting: { [qemu]: [Object.assign({}, unnotedFirst[qemu][1], { others: { "Virtual-1": qemu } })] }
            });
        // The connectors other screens' as a workspace leaves one, by their
        // descriptions as last heard ("" for one not heard yet): those
        // connected, and those left with it, each once, but the one it left.
        expect("the other screens as a workspace leaves one",
            Places.othersAt("Virtual-2", ["eDP-1", "Virtual-1", "Virtual-1", "Virtual-5"],
                ["Virtual-2", "Virtual-3"], { "eDP-1": panel, "Virtual-1": qemu, "Virtual-2": qemu,
                    "Virtual-3": qemu }),
            { "eDP-1": panel, "Virtual-1": qemu, "Virtual-5": "", "Virtual-3": qemu });

        // Windows that reopen themselves, matched by program and title,
        // else the first of that program, each kept window used once.
        const others = [
            { app: "google-chrome", title: "Docs" },
            { app: "google-chrome", title: "Open · Backwater" },
            { app: "slack", title: "Slack" }
        ];
        expect("a window is the kept one of its program with its title",
            Places.matchOther(others, [], "google-chrome", "Open · Backwater"), 1);
        expect("else the first of its program not used yet",
            Places.matchOther(others, [0], "google-chrome", "New Tab"), 1);
        expect("a program kept nowhere goes nowhere",
            Places.matchOther(others, [], "firefox", "Docs"), -1);
        expect("each kept window is used once",
            Places.matchOther(others, [2], "slack", "Slack"), -1);

        // Put back beside what's back: the session kept with Chrome on the
        // monitor's fifth came back as its fourth, since a workspace below
        // that held only other windows isn't made again.
        // A window Joel opened since sits first there, so it goes by its
        // neighbours, not by a column number.
        const session = (id, workspace, column, at) => ({
            id,
            at: { screen: monitor, output: "DP-4", workspace, column, tile: 1 },
            workspace: { id: 20 + at, idx: at, output: "DP-4" },
            column: column + 1
        });
        const beside = [session(1, 4, 1, 3), session(2, 5, 1, 4), session(3, 5, 3, 4),
            session(4, 5, 2, 4), session(5, 7, 1, 5)];
        const kept5 = column => ({ screen: monitor, output: "DP-4", workspace: 5, column, tile: 1 });
        expect("a window joins the workspace its session came back to, after the nearest left",
            Places.whereBack(kept5(3), beside), { join: 24, after: 4 });
        expect("before the nearest right when it sat first",
            Places.whereBack(kept5(1), beside.slice(2)), { join: 24, before: 4 });
        expect("a workspace that held no session is made after the one below it",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 6, column: 1, tile: 1 },
                beside), { make: true, output: "DP-4", index: 5 });
        expect("first on its screen when nothing sat below it",
            Places.whereBack({ screen: panel, output: "eDP-1", workspace: 2, column: 1, tile: 1 },
                beside), { make: true, output: null, index: 1 });
        // One kept below it that opened while every screen was gone sits on
        // a screen standing in for its own, whose numbers say nothing of
        // its own screen: after it there, the second of the monitor's
        // stayed on the laptop's once the monitor came back
        // (screens-keep-places, 2026-11-14).
        const standing = [{ id: 7, at: { screen: monitor, output: "DP-4", workspace: 1, column: 1,
            tile: 1 }, workspace: { id: 31, idx: 1, output: "eDP-1" }, column: 2 }];
        expect("one below it on a screen standing in for its own isn't followed there",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 2, column: 1, tile: 1 },
                standing), { make: true, output: null, index: 1 });
        expect("one kept under another goes under it",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 5, column: 2, tile: 2 },
                beside), { join: 24, after: 4, under: true });

        // Whether each window back is where it goes already, so the pass
        // after Reopen Them All moves only what Joel's focus or a screen
        // coming and going sent somewhere else.
        const now = (id, idx, column) => ({ workspace: { id: 20 + idx, idx, output: "DP-4" },
            column });
        const after4 = { join: 24, after: 4 };
        expect("a window just right of the one it goes after holds",
            Places.holds(after4, now(3, 4, 4), beside), true);
        expect("one left of it doesn't", Places.holds(after4, now(3, 4, 1), beside), false);
        expect("nor one on another workspace",
            Places.holds(after4, now(3, 3, 4), beside), false);
        expect("one under another holds in its column",
            Places.holds({ join: 24, after: 4, under: true }, now(3, 4, 3), beside), true);
        expect("one just left of the one it goes before holds",
            Places.holds({ join: 24, before: 4 }, now(3, 4, 2), beside), true);
        const made = { make: true, output: "DP-4", index: 5 };
        // A workspace's number is where it stands now, never what it is: one
        // emptied before it, or made, moves every number after it, so a
        // window alone on its own holds whatever number it has (P9R1KI,
        // 2026-11-09: 17 windows moved on screen at the 08:06 boot, chasing
        // a number the workspace before them had taken away).
        expect("one on a workspace of its own holds at its index or after",
            Places.holds(made, now(6, 6, 1), beside), true);
        expect("and before it, once a workspace before it has gone",
            Places.holds(made, now(6, 2, 1), beside), true);
        expect("nor among the others", Places.holds(made, now(6, 5, 1), beside), false);
        expect("a window on the workspace it joins goes just right of the one it follows",
            [Places.columnTo(after4, 1, beside), Places.columnTo(after4, 5, beside)], [3, 4]);
        expect("or just left of the one it goes before",
            [Places.columnTo({ join: 24, before: 4 }, 1, beside),
                Places.columnTo({ join: 24, before: 4 }, 5, beside)], [2, 3]);
        expect("a window opening on the workspace it joins opens just right of the one it follows",
            Places.opensAt(after4, beside), 4);
        expect("or in the column of the one it goes before, which moves right",
            Places.opensAt({ join: 24, before: 4 }, beside), 3);
        expect("one going under another opens just right of it, to be taken in after",
            Places.opensAt({ join: 24, after: 4, under: true }, beside), 4);
        expect("one given a workspace of its own opens at none", Places.opensAt(made, beside),
            null);

        // A session its orchestrator hands over goes where it sat this boot,
        // else where the restored boot kept it.
        const here = { output: "eDP-1", workspace: 1, column: 2, tile: 1 };
        const restoredAt = { output: "DP-4", workspace: 5, column: 1, tile: 1 };
        const keptPlaces = { now: { "01-a": here },
            restored: { "01-a": restoredAt, "01-b": restoredAt } };
        expect("handed over: this boot's, the restored boot's, none",
            ["01-a", "01-b", "01-c"].map(id => Places.keptFor(id, keptPlaces)),
            [here, restoredAt, null]);
        // Which boot's each was, so placed knows which were kept at once.
        expect("handed over from: this boot, the restored boot, none",
            ["01-a", "01-b", "01-c"].map(id => Places.keptFrom(id, keptPlaces)), ["now", "restored", ""]);
        // A session started under an orchestrator opens on the orchestrator's
        // workspace, in title order after it, wherever the session that
        // started it sits; one under it elsewhere doesn't count.
        const lead = { workspace: 7, column: 2, title: "Reef Orchestrator" };
        const under = [{ workspace: 7, column: 3, title: "Anchor Checks" },
            { workspace: 7, column: 4, title: "Kelp Survey" },
            { workspace: 5, column: 1, title: "Buoy Lights" }];
        expect("under an orchestrator, by title: between, first, last, untitled last",
            ["Coral Counts", "Abalone", "Zebra Mussels", ""]
                .map(title => Places.underOrchestrator(title, lead, under)),
            [{ workspace: 7, column: 4 }, { workspace: 7, column: 3 },
                { workspace: 7, column: 5 }, { workspace: 7, column: 5 }]);
        expect("with no orchestrator here, nowhere of its own",
            Places.underOrchestrator("Coral Counts", null, under), null);
        // A session goes home to its orchestrator's workspace, by the
        // address of the top orchestrator over its quest as the harness
        // hands it over, else the orchestrator it names, else the one its
        // place last boot was kept under; never by a word that's no address.
        expect("home: handed over, said, kept, none, and no address",
            [Places.homeOf({ workspace: "REEF", orchestratedBy: "KELP" }, { under: "BUOY" }),
                Places.homeOf({ orchestratedBy: "KELP" }, { under: "BUOY" }),
                Places.homeOf({ id: "01-a" }, { under: "BUOY" }),
                Places.homeOf({ id: "01-a" }, null),
                Places.homeOf({ workspace: "reef; rm" }, null)],
            ["REEF", "KELP", "BUOY", "", ""]);
        // An orchestrator back at its kept place names that workspace for
        // itself, so a session of its own back before its pi says who it is
        // joins it there, rather than making a workspace of its own and
        // staying alone on it (Joel, 2026-11-14 22:36: "why did this window
        // get put on a workspace by itself?").
        const spaces = [{ id: 3, name: null }, { id: 5, name: "KELP" }];
        expect("named for its orchestrator: by address, by what it orchestrates",
            [Places.homeToName({ address: "REEF" }, "REEF", 3, spaces),
                Places.homeToName({ orchestrates: "REEF" }, "REEF", 3, spaces)],
            ["REEF", "REEF"]);
        expect("named for nothing: one it leads, its own name, another's, nowhere, no home",
            [Places.homeToName({ id: "01-a" }, "REEF", 3, spaces),
                Places.homeToName({ address: "KELP" }, "KELP", 3, spaces),
                Places.homeToName({ address: "REEF" }, "REEF", 5, spaces),
                Places.homeToName({ address: "REEF" }, "REEF", null, spaces),
                Places.homeToName({ address: "REEF" }, "", 3, spaces)],
            ["", "", "", "", ""]);
        // After the restart on 2026-11-21, WM4KSZ's eleven sessions came
        // back on workspace 3 by its number, its name gone to an empty
        // fifth: named workspaces come back from a boot in another order.
        expect("kept on a named workspace, it goes back to the one named so now",
            Reopening.namedBack({ workspace: 3, workspaceName: "KELP" }, spaces), { join: 5 });
        expect("none named so now, the workspace it lands on takes the name",
            Reopening.namedBack({ workspace: 3, workspaceName: "REEF" }, spaces),
            { name: "REEF" });
        expect("kept on one named nothing, it goes by its number",
            [Reopening.namedBack({ workspace: 3 }, spaces),
                Reopening.namedBack({ workspace: 3, workspaceName: "" }, spaces),
                Reopening.namedBack(null, spaces)], [null, null, null]);
        // Only the empty workspace it found takes the kept name, and only
        // while unnamed: the last with windows may be one Joel named.
        const emptied = { id: 9 };
        const mine = { id: 9, name: "MINE" };
        expect("the kept name goes only to the empty, unnamed workspace found",
            [Reopening.keptName({ name: "REEF" }, emptied, emptied),
                Reopening.keptName({ name: "REEF" }, { id: 4 }, emptied),
                Reopening.keptName({ name: "REEF" }, { id: 4, name: "MINE" }, null),
                Reopening.keptName({ name: "REEF" }, mine, mine),
                Reopening.keptName({ join: 5 }, emptied, emptied),
                Reopening.keptName(null, emptied, emptied)],
            ["REEF", "", "", "", "", ""]);
        // One sent by name to a workspace on another screen than it was
        // kept on stays there, or placing it would pull it off again.
        expect("one on the workspace named as kept holds there, on any screen",
            [Reopening.onNamed({ output: "DP-4", workspaceName: "KELP" },
                { id: 5, name: "KELP", output: "eDP-1" }),
                Reopening.onNamed({ output: "DP-4", workspaceName: "KELP" },
                    { id: 6, name: "REEF", output: "DP-4" }),
                Reopening.onNamed({ output: "DP-4" }, { id: 7, output: "DP-4" }),
                Reopening.onNamed({ workspaceName: "" }, { id: 8, name: "" }),
                Reopening.onNamed(null, { id: 5, name: "KELP" })],
            [true, false, false, false, false]);
        // One handed over while Reopen Them All already holds it, or has
        // begun it, joins nothing, or two pis would write one log.
        const asked = id => ({ one: { id } });
        expect("handed over again while queued or begun, it isn't asked twice",
            Reopening.queuedOnce([asked("q1"), asked("q2")], [asked("q2"), asked("q3"), asked("q4")],
                ["q4"]).map(entry => entry.one.id), ["q1", "q2", "q3"]);
        // What Reopen Them All's lists find gone, counted by the walk
        // Sessions joins each to: its own while it may still find one of
        // its tally's folders gone, else one of the list's own. Each
        // reading is what Agents' foot says next.
        const heard = [];
        const hear = now => heard.push(`${now.gone} gone, ${now.reopened} reopened`);
        const walkOf = (tally, queue, looking, begun) => ({ tally, queue, looking,
            begun: begun ?? [] });
        const walked = Reopening.tallyOf(null, ["a", "b", "c"], hear);
        Reopening.foundGone(walked, { id: "a" });
        expect("a list with no walk counts its own", heard, ["1 gone, 2 reopened"]);
        // A second given while the walk still holds sessions joins it, the
        // same sessions counting once and one more among them, told at once
        // since one was told gone already.
        const walking = walkOf(walked, [asked("b")], null);
        expect("given mid-walk, it counts in the walk's tally",
            Reopening.tallyOf(walking, ["a", "b", "c", "d"], hear) === walked, true);
        Reopening.foundGone(walked, { id: "d" });
        Reopening.foundGone(walked, { id: "b" });
        expect("and the walk's count grows by what it adds", heard.slice(1),
            ["1 gone, 3 reopened", "2 gone, 2 reopened", "3 gone, 1 reopened"]);
        expect("given while it holds one started afresh for its own, it joins",
            Reopening.tallyOf(walkOf(walked, [{ one: { id: "fresh", afreshFor: "c" } }], null),
                [], hear) === walked, true);
        expect("given while the walk looks for its last folder, it joins too",
            Reopening.tallyOf(walkOf(walked, [], asked("c")), ["a"], hear) === walked, true);
        // Once every session it asked for is begun, nothing more of its can
        // be found gone, though a window given up on keeps the walk for good
        // or the harness's sessions keep it busy, so a list given then
        // counts afresh (core.nix#6037's F39).
        expect("given while only the harness's sessions keep the walk busy, it counts afresh",
            Reopening.tallyOf(walkOf(walked, [asked("h")], asked("h2")), ["x"], hear)
                === walked, false);
        const after = Reopening.tallyOf(walkOf(walked, [], null), ["a", "b", "c"], hear);
        Reopening.foundGone(after, { id: "a" });
        expect("given once the walk can find no more gone, it counts afresh",
            [after === walked, heard[heard.length - 1]], [false, "1 gone, 2 reopened"]);
        // Counting afresh on a walk that began some of what it's given, it
        // counts none of those, which the walk won't look for again, one
        // found gone among them.
        const later = Reopening.tallyOf(walkOf(walked, [], null, ["a", "b", "c"]),
            ["a", "b", "c", "d"], hear);
        Reopening.foundGone(later, { id: "d" });
        expect("given once the walk began some of it, it counts only the rest",
            heard[heard.length - 1], "1 gone, 0 reopened");
        // Joining a tally still counting, as a press again while one of its
        // sessions waits its turn, it counts none of those either.
        const second = Reopening.tallyOf(walkOf(walked, [], null, ["a", "b", "c"]),
            ["a", "b", "c", "d", "e"], hear);
        const joining = Reopening.tallyOf(walkOf(second, [asked("e")], null, ["a", "b", "c", "d"]),
            ["a", "b", "c", "d", "e"], hear);
        Reopening.foundGone(second, { id: "e" });
        expect("joined while it counts, it counts none the walk is done with",
            [joining === second, heard[heard.length - 1]], [true, "1 gone, 1 reopened"]);
        // One the walk began and can still find gone, its folder looked for
        // now or one queued afresh for it, counts, so its folder gone is told.
        const still = Reopening.tallyOf(walkOf(null, [{ one: { id: "fresh", afreshFor: "y" } }],
            asked("c"), ["c", "y"]), ["c", "y", "x"], hear);
        Reopening.foundGone(still, { id: "c" });
        Reopening.foundGone(still, { id: "fresh", afreshFor: "y" });
        expect("begun and still looked for, or queued afresh, it counts once found gone",
            heard.slice(-2), ["1 gone, 2 reopened", "2 gone, 1 reopened"]);
        // A walk the harness began counts nothing, and a list given during
        // it counts the sessions both hold, whichever list queued them.
        const harness = walkOf(null, [asked("h"), asked("a")], null);
        const during = Reopening.tallyOf(harness, ["a", "b"], hear);
        Reopening.foundGone(during, { id: "a" });
        expect("given during the harness's walk, a session it asked for counts once found gone",
            heard[heard.length - 1], "1 gone, 1 reopened");
        // One started afresh counts as the one it stands in for, and one
        // no list asked for counts nowhere, telling nothing.
        const soFar = heard.length;
        Reopening.foundGone(during, { id: "fresh", afreshFor: "b" });
        Reopening.foundGone(during, { id: "h" });
        Reopening.foundGone(null, { id: "a" });
        expect("one started afresh counts as the one it stands for, one not asked nowhere",
            heard.slice(soFar), ["2 gone, 0 reopened"]);
        // Those whose kept place says they led others are begun first, so
        // the sessions they lead have them back to answer to; the rest,
        // those kept nowhere among them, keep their order.
        const queued = (id, leads) => ({ one: { id },
            kept: leads === undefined ? null : Object.assign({ leads }, here) });
        expect("orchestrators first, each part in its order",
            Reopening.leadersFirst([queued("a", false), queued("b", true), queued("c"),
                queued("d", true), queued("e", false)]).map(entry => entry.one.id),
            ["b", "d", "a", "c", "e"]);

        // After the machine reset on 2026-11-20, Reopen Them All gave four
        // sessions kept nowhere, which ran with no window, windows on their
        // orchestrator's workspace, and sent one kept on the second
        // workspace to its orchestrator's third.
        const keptHere = Object.assign({ leads: false }, here);
        // A window goes back by the screen in use as its sessions were
        // placed, while that's connected, else by the one in use now.
        const twoThere = { "eDP-1": panel, "DP-4": monitor };
        expect("a window goes by the screen in use as its sessions were placed",
            Reopening.inUseFor("DP-4", "eDP-1", twoThere), "DP-4");
        expect("a window goes by the screen in use now once that one is gone",
            Reopening.inUseFor("DP-5", "eDP-1", twoThere), "eDP-1");
        expect("with none in use as its sessions were placed, a window goes by the one now",
            [Reopening.inUseFor(null, "eDP-1", twoThere),
                Reopening.inUseFor(undefined, "eDP-1", twoThere)], ["eDP-1", "eDP-1"]);
        expect("a prototype's name is never the screen in use a window goes by",
            Reopening.inUseFor("constructor", "eDP-1", twoThere), "eDP-1");
        expect("kept somewhere, it opens there, not at its orchestrator's",
            Reopening.broughtBack(true, keptHere, true, true), { window: true, home: false });
        expect("kept nowhere, it comes back held without a window",
            Reopening.broughtBack(true, null, true, true), { window: false, home: true });
        // A window the agents daemon saw but couldn't place, one floating
        // or one its kitty drew beside another, is kept as unplaced.
        expect("kept with a window it couldn't place, it comes back in one, going home",
            Reopening.broughtBack(true, { unplaced: true }, true, true),
            { window: true, home: true });
        expect("and opens at no place, one kept somewhere at its own",
            [Reopening.placeOf({ unplaced: true }), Reopening.placeOf(keptHere),
                Reopening.placeOf(null)], [null, keptHere, null]);
        expect("one sat here only as the last boot's",
            [{ way: "boot" }, { way: "carried" }, {}, null].map(Reopening.satHere),
            [true, false, false, false]);
        // Each of the walk's ways opens one at the same place, its workspace's
        // name kept, so none goes back by number alone.
        expect("a place opened at keeps its workspace's name",
            Reopening.atOf({ output: "DP-4", workspace: 3, workspaceName: "KELP", column: 2,
                screen: "HP", leads: true }),
            { output: "DP-4", workspace: 3, workspaceName: "KELP", column: 2, tile: 1 });
        expect("none to open at, none", Reopening.atOf(null), null);
        expect("with no places known, none is taken for windowless",
            Reopening.broughtBack(true, null, false, true), { window: true, home: true });
        expect("carried or given back, never kept here, it opens as it did",
            Reopening.broughtBack(true, null, true, false), { window: true, home: true });
        expect("handed over or started alone, it opens as it did, going home",
            [Reopening.broughtBack(false, null, true, true),
                Reopening.broughtBack(false, keptHere, true, true)],
            [{ window: true, home: true }, { window: true, home: true }]);
        // Nothing on screen says one started without a window, so it isn't
        // begun again until its pi has had time to say it runs.
        const startedAt = { "held-1": 1000 };
        expect("started without a window moments ago, it isn't begun again",
            Reopening.startedHeldLately(startedAt, "held-1", 1000 + 60 * 1000), true);
        expect("long since, or never, it may be",
            [Reopening.startedHeldLately(startedAt, "held-1", 1000 + 5 * 60 * 1000),
                Reopening.startedHeldLately(startedAt, "held-2", 2000),
                Reopening.startedHeldLately(undefined, "held-1", 2000)],
            [false, false, false]);

        // Reopen Them All begins the next session only while the machine has
        // room for it, as the machine queue judges it, so eighty sessions
        // never start into a machine already short of memory or disk.
        const queue = (over, io, heartbeat) => JSON.stringify({ v: 1,
            heartbeat: heartbeat ?? 5000,
            machine: { over, pressure: { cpu: 30, memory: 2, io } } });
        expect("room: the next begins", Reopening.mayBegin(queue([], 10), 5010), true);
        expect("memory over: it waits", Reopening.mayBegin(queue(["memory"], 10), 5010), false);
        expect("processors over: it waits", Reopening.mayBegin(queue(["cpu"], 10), 5010), false);
        expect("the disk stalling: it waits", Reopening.mayBegin(queue([], 60), 5010), false);
        // A queue that isn't heard from says nothing of the machine, so it
        // never holds a reopening for good.
        expect("a queue gone quiet holds nothing",
            Reopening.mayBegin(queue(["memory"], 90, 5000), 5000 + 121), true);
        expect("no queue holds nothing",
            [Reopening.mayBegin("", 5010), Reopening.mayBegin("{", 5010)], [true, true]);

        // Thirty reopened at once each open as before, but are told to
        // carry on only as the uplink has room, since a session told to
        // carry on is what sends: on 2026-11-14 seventy-two told at once
        // filled the home line's upload for a quarter of an hour.
        {
        const thirty = Array.from({ length: 30 }, (_, at) => ({ id: `01-${at}`,
            address: `S${at}`, carried: at === 20 ? { from: "far", at: "1" } : undefined,
            leads: at === 25 }));
        const sending = (mbit, heartbeat) => JSON.stringify({ v: 1, heartbeat: heartbeat ?? 5000,
            machine: { over: [], pressure: { io: 1 }, uplink: mbit === undefined ? undefined
                : { sentMbit: mbit } } });
        const ids = told => told.map(one => one.id);
        const due = (text, told) => Reopening.carryOnsDue(thirty, told, text, 5010);
        expect("the line full: none is told yet", ids(due(sending(6.1), [])), []);
        expect("the line with room: one at a time", due(sending(2.0), []).length, 1);
        // Those waiting on Joel first, then one carried in, then a leader.
        const wanting = thirty.map(one => Object.assign({}, one, { wants: one.id === "01-7" }));
        expect("waiting on Joel, then carried in, then leaders, then the rest",
            ids(Reopening.carryOnsDue(wanting, [], sending(undefined), 5010)),
            ["01-7", "01-20", "01-25", "01-0"]);
        // With no uplink said yet, four at a time, the next once one has
        // gone idle or a minute has passed since it was told.
        const told = (at, idle) => thirty.slice(0, 4).map(one => ({ id: one.id, at, idle }));
        expect("four working: none more", due(sending(undefined), told(5000, false)).length, 0);
        expect("one gone idle: one more", due(sending(undefined),
            told(5000, false).map((one, at) => at === 0 ? Object.assign({}, one, { idle: true })
                : one)).length, 1);
        expect("a minute on: four more", due(sending(undefined), told(4940, false)).length, 4);
        // A queue not heard from says nothing of the line, so all are told.
        expect("a queue gone quiet: all told at once",
            due(sending(6.1, 5010 - 121), []).length, 30);
        expect("no queue: all told at once", Reopening.carryOnsDue(thirty, [], "", 5010)
            .length, 30);
        // Before the line has room again, no more than the few allowed go:
        // thirty pending through ten looks at a full line, none told.
        let sent = [];
        for (let look = 0; look < 10; look += 1)
            sent = sent.concat(due(sending(5.0), sent));
        expect("thirty reopened, the line full: none told before room returns", sent.length, 0);
        let few = [];
        for (let look = 0; look < 10; look += 1)
            few = few.concat(due(sending(undefined), few)
                .map(one => ({ id: one.id, at: 5010, idle: false })));
        expect("thirty reopened, no uplink said: four told until one is done", few.length, 4);
        }

        // A session moving onto pi's host keeps where its window was, as
        // the windows daemon's record of it says, so its viewer opens there
        // once it runs hosted; one with no window, or none laid out yet,
        // keeps nothing.
        const laid = (workspace, column) => ({ id: 7, workspace, column, down: 1 });
        expect("a window's place kept", Viewers.placeOf(laid(4, 3), 100),
            { workspace: 4, column: 3, at: 100 });
        expect("a window floating keeps its workspace alone",
            Viewers.placeOf(laid(4, null), 100), { workspace: 4, column: null, at: 100 });
        expect("no window keeps nothing", [Viewers.placeOf(null, 100),
            Viewers.placeOf(laid(null, null), 100)], [null, null]);

        // A viewer's window is titled as pi's host will title it, from the
        // first moment, never for the command it runs: what the session's
        // record says, else its quest's name and its folder, or its folder
        // alone where its name is its folder, as one with no quest is named.
        expect("a viewer titled as its record says",
            Viewers.viewerTitle("\u03c0 - Reef - harbour", "Shoal", "/home/skiff/keel"),
            "\u03c0 - Reef - harbour");
        expect("a viewer titled by its quest and folder",
            Viewers.viewerTitle(undefined, "Reef", "/home/skiff/harbour/"),
            "\u03c0 - Reef - harbour");
        expect("a viewer named for its folder titled by its folder once",
            Viewers.viewerTitle("", "harbour", "/home/skiff/harbour"), "\u03c0 - harbour");
        expect("a record's title that isn't pi's is no viewer's",
            Viewers.viewerTitle("mycelium", "Reef", "/home/skiff/harbour"),
            "\u03c0 - Reef - harbour");
        expect("words titled first, the title one word, said to nothing but the title",
            Viewers.titled("\u03c0 - Reef\u001b]2;x\u0007\u007f\u009b\u00a0",
                ["mycelium", "attach"]),
            ["sh", "-c", 'printf "\\033]2;%s\\007" "$0"; exec "$@"',
                "\u03c0 - Reef ]2;x   \u00a0", "mycelium", "attach"]);
        // A hosted session's restore entry says the words its viewer runs
        // while its host isn't serving it yet; an entry an older harness
        // wrote says none, and words not whole are none.
        const viewWords = ["node", "/lasting/host-join.ts", "--view", "--session", "s-a"];
        expect("a hosted entry's view words", Viewers.waitingWords({ view: viewWords }),
            viewWords);
        expect("no view words, or none whole, are none", [Viewers.waitingWords({}),
            Viewers.waitingWords({ view: [] }), Viewers.waitingWords({ view: ["node", 3] }),
            Viewers.waitingWords(null)], [null, null, null, null]);
        // A row's Reopen is said by the row's name as the daemon words it,
        // its id only where the row has no name to say.
        expect("a row is said by its name", Viewers.rowSaid("s-a", { name: "Reef" }),
            { address: "Reef", machine: "" });
        expect("one with an empty name, or none, by its id", [Viewers.rowSaid("s-a",
            { name: "" }).address, Viewers.rowSaid("s-a", {}).address,
            Viewers.rowSaid("s-a", undefined).address], ["s-a", "s-a", "s-a"]);

        // Each kept place is one viewer to open once its session runs
        // hosted with words to view it by, and then kept no more; one whose
        // viewer is open already is let go, and one a day old lapses, as
        // one whose session never moved would.
        const day = 24 * 60 * 60 * 1000;
        const hosted = (id, window) => ({ window: window ?? null, attach: ["attach", id],
            presence: { session: id, hosted: { host: "harbour-attended" } } });
        const plain = id => ({ window: laid(1, 1), attach: null, presence: { session: id } });
        const moving = { moved: { workspace: 4, column: 3, at: 0 },
            moving: { workspace: 5, column: 1, at: 0 },
            viewed: { workspace: 6, column: 2, at: 0 },
            stale: { workspace: 7, column: 1, at: -day } };
        // niri's workspaces by id: Joel on 9 of the monitor, the laptop's
        // 4 to 7 behind it, and the empty one niri keeps last on each.
        const workspace = (id, idx, output, windowId) => ({ id, idx, output,
            is_focused: false, is_active: false, active_window_id: windowId });
        const on = (focusedId, all) => {
            const now = {};
            for (const one of Object.values(all))
                now[one.id] = Object.assign({}, one, { is_focused: one.id === focusedId });
            return now;
        };
        const screens = on(9, {
            4: workspace(4, 1, "eDP-1", 40), 5: workspace(5, 2, "eDP-1", 50),
            6: workspace(6, 3, "eDP-1", 60), 7: workspace(7, 4, "eDP-1", 70),
            8: workspace(8, 5, "eDP-1", null),
            9: workspace(9, 1, "DP-2", 90), 10: workspace(10, 2, "DP-2", null) });
        // Kept since the shell started a day ago, with Joel away, unless a
        // case says otherwise.
        const gone = (all, said) => Object.assign({ workspaces: all, since: -day, away: true },
            said ?? {});
        const due = Viewers.viewersDue(moving, [hosted("moved"), plain("moving"),
            hosted("viewed", laid(6, 2)), hosted("stale")], 10, gone(screens));
        expect("the moved one's viewer opens where it was",
            due.open.map(one => [one.session.presence.session, one.into]),
            [["moved", { workspace: 4, column: 3 }]]);
        expect("only the one still moving stays kept", due.kept,
            { moving: { workspace: 5, column: 1, at: 0 } });
        expect("one hosted with no words waits", Viewers.viewersDue(moving,
            [Object.assign(hosted("moved"), { attach: [] })], 10, gone(screens)).kept.moved,
            { workspace: 4, column: 3, at: 0 });
        expect("nothing kept, nothing to open", Viewers.viewersDue({}, [hosted("moved")], 10,
            gone(screens)), { open: [], kept: {} });

        // Nothing opens by itself while Joel is at the machine, nothing kept
        // before this shell started, as one restarted by a switch finds,
        // and nothing parked in its host, which viewing would wake: each
        // waits, kept (OSPHT4 [0762284c], 13:41:38 on 11 November).
        const alone = { moved: { workspace: 4, column: 3, at: 0 } };
        expect("with Joel there it waits", Viewers.viewersDue(alone, [hosted("moved")], 10,
            gone(screens, { away: false })), { open: [], kept: alone });
        expect("kept before the shell started, it waits", Viewers.viewersDue(alone,
            [hosted("moved")], 10, gone(screens, { since: 5 })), { open: [], kept: alone });
        const parked = Object.assign(hosted("moved"), { presence: Object.assign(
            {}, hosted("moved").presence, { parked: { since: "2026-11-11T09:00:00Z" } }) });
        expect("parked in its host, it waits", Viewers.viewersDue(alone, [parked], 10,
            gone(screens)), { open: [], kept: alone });
        expect("its viewer open, it is let go however Joel is", Viewers.viewersDue(alone,
            [hosted("moved", laid(4, 3))], 10, gone(screens, { away: false })),
            { open: [], kept: {} });
        // A window the windows daemon's index still names, which niri, its
        // list trusted, no longer lists, as in a gap of the daemon's, is no
        // viewer open (Sessions.qml's listedNow): its viewer opens.
        // Niri lists every window but the session's own (7, as laid), and
        // no window is never listed, as listedNow says.
        const unlisted = gone(screens, { listed: window => Boolean(window) && window.id !== 7 });
        expect("its window named by the index alone, it opens", Viewers.viewersDue(alone,
            [hosted("moved", laid(4, 3))], 10, unlisted).open.map(one => one.into),
            [{ workspace: 4, column: 3 }]);
        const woken = { moved: { workspace: 4, column: 3, at: 0, slept: true, asleep: true,
            asked: true } };
        expect("a sleeper woken, its window named by the index alone, opens",
            Viewers.viewersDue(woken, [hosted("moved", laid(4, 3))], 5, unlisted).open
                .map(one => one.into), [{ workspace: 4, column: 3 }]);

        // One Reopen Them All kept while Joel was on its workspace, with
        // words to wait for its host by, opens by them once he isn't
        // there, its host serving it or not, him away or not, since he
        // asked for it; not where he is, nor if kept before this shell
        // started (20 November: kept for its host alone, it opened by
        // itself nearly two hours later).
        const words = ["node", "/lasting/scripts/host-join.ts", "--view", "--session", "held"];
        const keptAsked = (workspace, more) => ({ held: Object.assign({ workspace, column: 2,
            at: 0, waiting: words, named: "pi-hosted.held", title: "\u03c0 - Reef" },
            more ?? {}) });
        expect("its host away and Joel elsewhere, it opens by its words",
            Viewers.viewersDue(keptAsked(4), [], 10, gone(screens, { away: false })),
            { open: [{ id: "held", place: keptAsked(4).held, into: { workspace: 4, column: 2 } }],
                kept: {} });
        expect("on Joel's workspace it waits", Viewers.viewersDue(keptAsked(9), [], 10,
            gone(screens)), { open: [], kept: keptAsked(9) });
        expect("kept before the shell started, it waits", Viewers.viewersDue(keptAsked(4), [], 10,
            gone(screens, { since: 5 })), { open: [], kept: keptAsked(4) });
        expect("words that aren't all words, it waits for its host",
            Viewers.viewersDue(keptAsked(4, { waiting: ["node", 7] }), [], 10, gone(screens)),
            { open: [], kept: keptAsked(4, { waiting: ["node", 7] }) });
        expect("its host serving it, it opens as any served one",
            Viewers.viewersDue(keptAsked(4), [hosted("held")], 10, gone(screens)).open
                .map(one => [one.session.presence.session, one.into]),
            [["held", { workspace: 4, column: 2 }]]);
        expect("its host back before Joel left, it opens by its session's words, him not away",
            Viewers.viewersDue(keptAsked(4), [hosted("held")], 10,
                gone(screens, { away: false })).open
                .map(one => [one.session.presence.session, one.into]),
            [["held", { workspace: 4, column: 2 }]]);
        const napped = hosted("held");
        napped.presence.parked = { since: "then" };
        expect("parked in its host, it waits, since viewing it would wake it",
            Viewers.viewersDue(keptAsked(4), [napped], 10, gone(screens)),
            { open: [], kept: keptAsked(4) });
        expect("asked past ASKED_FOR, it opens none and is let go",
            Viewers.viewersDue(keptAsked(4), [], Viewers.ASKED_FOR + 1, gone(screens)),
            { open: [], kept: {} });
        expect("back as a plain pi, whose window it would never be, it's let go",
            Viewers.viewersDue(keptAsked(4), [plain("held")], 10, gone(screens)),
            { open: [], kept: {} });
        // Kept while Joel was on its workspace, ASKED_FOR doesn't run while he
        // stays there, since he may work there for hours: once he leaves,
        // it opens, served or not and him away or not, or ASKED_FOR runs
        // from then if nothing can open it yet.
        const keptThere = (workspace, more) => keptAsked(workspace,
            Object.assign({ asked: true, there: true }, more ?? {}));
        const lateLeft = Viewers.ASKED_FOR + 1;
        expect("Joel on its workspace past ASKED_FOR, it waits",
            Viewers.viewersDue(keptThere(9), [], lateLeft, gone(screens)),
            { open: [], kept: keptThere(9) });
        expect("and once he leaves, however late, it opens by its words",
            Viewers.viewersDue(keptThere(4), [], lateLeft, gone(screens, { away: false }))
                .open.map(one => one.into), [{ workspace: 4, column: 2 }]);
        expect("served, with no words of its own, it opens by its session's once he leaves",
            Viewers.viewersDue({ held: { workspace: 4, column: 2, at: 0, asked: true,
                there: true } }, [hosted("held")], lateLeft, gone(screens, { away: false }))
                .open.map(one => [one.session.presence.session, one.into]),
            [["held", { workspace: 4, column: 2 }]]);
        const leftUnviewable = Viewers.viewersDue({ held: { workspace: 4, column: 2, at: 0,
            asked: true, there: true } }, [], lateLeft, gone(screens)).kept.held;
        expect("left with nothing to open it by yet, ASKED_FOR runs from then",
            [leftUnviewable?.at, leftUnviewable?.there], [lateLeft, undefined]);
        expect("parked in its host, it's viewed once he leaves, which wakes it, as he asked",
            Viewers.viewersDue(keptThere(4), [napped], lateLeft, gone(screens)).open
                .map(one => [one.session.presence.session, one.into]),
            [["held", { workspace: 4, column: 2 }]]);
        expect("kept by a shell before this one, it lapses from the ask all the same",
            Viewers.viewersDue(keptThere(9), [], lateLeft, gone(screens, { since: 5 })),
            { open: [], kept: {} });

        // A viewer never opens where Joel is, so it never takes his focus
        // or flashes beside him: on the workspace he's on it waits until he
        // leaves, and one whose workspace went with its window waits rather
        // than taking another (OSPHT4 [0762284c]).
        const place = (workspace, column) => ({ moved: { workspace, column, at: 0 } });
        const opened = (kept, all) => Viewers.viewersDue(kept, [hosted("moved")], 10, gone(all));
        expect("on Joel's workspace it waits", opened(place(9, 2), screens),
            { open: [], kept: place(9, 2) });
        expect("once he leaves, it opens there",
            opened(place(9, 2), on(4, screens)).open.map(one => one.into),
            [{ workspace: 9, column: 2 }]);
        expect("its workspace gone, it waits", opened(place(3, 1), screens),
            { open: [], kept: place(3, 1) });
        expect("no workspaces known, it waits", opened(place(4, 3), {}).kept, place(4, 3));

        // A session put to sleep has no window, and waking it opens none:
        // a window opens only when Joel asks for one (Joel, 16 November:
        // "i would love for any mycelial node to not require a live
        // window"). Only Reopen Them All asks, so a sleeper it wakes opens
        // where it was, in pi's host or in its holder, with Joel there too,
        // but never on his workspace, never while it sleeps, and only
        // within ASKED_FOR of his asking. Kept just before its window
        // closes, it waits to be seen asleep, and lapses in a day if it
        // never is; once seen, it's kept however long it sleeps, unless it
        // was asked for, which lapses ASKED_FOR after the ask.
        const slept = { napper: { workspace: 4, column: 3, at: 0, slept: true } };
        const asleep = { napper: { workspace: 4, column: 3, at: 0, slept: true, asleep: true } };
        const reopened = { napper: { workspace: 4, column: 3, at: 0, slept: true, asleep: true,
            asked: true } };
        // Joel at the machine, with the shell started after the place was kept.
        const there = gone(screens, { away: false, since: 5 });
        const napping = (session, extra) => Object.assign(session, { presence: Object.assign(
            {}, session.presence, extra) });
        const hostedNapping = napping(hosted("napper"), { parked: { since: "2026-11-14" } });
        const held = (window, state) => ({ window: window ?? null, attach: null, held: "h1",
            presence: { session: "napper", state } });
        expect("its window still open, not yet asleep, it waits", Viewers.viewersDue(slept,
            [hosted("napper", laid(4, 3))], 10, there), { open: [], kept: slept });
        expect("its window gone before the host parked it, it waits", Viewers.viewersDue(slept,
            [hosted("napper")], 10, there), { open: [], kept: slept });
        expect("asleep in its host, it's marked so and waits", Viewers.viewersDue(slept,
            [hostedNapping], 10, there), { open: [], kept: asleep });
        expect("asleep in its holder, the same", Viewers.viewersDue(slept,
            [held(null, "parked")], 10, there), { open: [], kept: asleep });
        expect("woken in its host, unasked, it opens no window and is let go",
            Viewers.viewersDue(asleep, [hosted("napper")], 10, there), { open: [], kept: {} });
        expect("woken in its holder, unasked, the same", Viewers.viewersDue(asleep,
            [held(null, "listening")], 10, there), { open: [], kept: {} });
        expect("woken by Reopen Them All in its host, it opens where it was",
            Viewers.viewersDue(reopened, [hosted("napper")], 10, there).open
                .map(one => [one.session.presence.session, one.into]),
            [["napper", { workspace: 4, column: 3 }]]);
        expect("woken by Reopen Them All in its holder, the same", Viewers.viewersDue(reopened,
            [held(null, "listening")], 10, there).open.map(one => [one.session.held, one.into]),
            [["h1", { workspace: 4, column: 3 }]]);
        expect("woken past ASKED_FOR of the ask, it opens none and is let go",
            Viewers.viewersDue(reopened, [held(null, "listening")], Viewers.ASKED_FOR + 1, there),
            { open: [], kept: {} });
        expect("asked and still asleep past ASKED_FOR, it's let go", Viewers.viewersDue(reopened,
            [hostedNapping], Viewers.ASKED_FOR + 1, there), { open: [], kept: {} });
        const askedLater = { napper: Object.assign({}, reopened.napper, { at: 5000 }) };
        expect("asked later, ASKED_FOR runs from the ask, not from nothing",
            Viewers.viewersDue(askedLater, [held(null, "listening")], 5000 + Viewers.ASKED_FOR,
                there).open.length, 1);
        expect("and past it from the ask, it's let go", Viewers.viewersDue(askedLater,
            [held(null, "listening")], 5001 + Viewers.ASKED_FOR, there), { open: [], kept: {} });
        const askedWhenUnread = { napper: Object.assign({}, reopened.napper, { at: "soon" }) };
        expect("asked when nobody can read, it opens none and is let go",
            Viewers.viewersDue(askedWhenUnread, [held(null, "listening")], 10, there),
            { open: [], kept: {} });
        expect("woken while Joel is on its workspace, it waits", Viewers.viewersDue(reopened,
            [hosted("napper")], 10, gone(on(4, screens), { away: false })),
            { open: [], kept: reopened });
        expect("woken with no words to view it by yet, it waits", Viewers.viewersDue(reopened,
            [Object.assign(hosted("napper"), { attach: [] })], 10, there),
            { open: [], kept: reopened });
        expect("woken and opened already, as by Go To, it's let go", Viewers.viewersDue(reopened,
            [hosted("napper", laid(5, 1))], 10, there), { open: [], kept: {} });
        expect("asleep two days, it's still kept", Viewers.viewersDue(asleep,
            [hostedNapping], 2 * day, there).kept, asleep);
        expect("asleep and unlisted, as its host starts again, it's kept",
            Viewers.viewersDue(asleep, [], 2 * day, there).kept, asleep);
        expect("never seen asleep, it lapses in a day", Viewers.viewersDue(slept,
            [hosted("napper")], 2 * day, there), { open: [], kept: {} });

        // Reopen Them All begins no session a pi runs here, wakes one
        // parked here, and believes a hosted presence only while its host's
        // own list names it (OSPHT4 [58d7fa8e], 15 November).
        const owner = { pid: 4100, startedAt: 17, bootId: "b1", startTicks: 7,
            host: "work-background", session: "s-a" };
        const list = (fields, ids) => JSON.stringify(Object.assign({ v: 1,
            key: "work-background", owner: { pid: 4100, startedAt: 17, bootId: "b1",
                startTicks: 7 }, sessions: ids.map(id => ({ id, state: "idle" })) }, fields));
        expect("a host's list named by its pid and start ticks", Places.hostListOf(owner),
            "4100-7.json");
        expect("by its start, without ticks", Places.hostListOf({ pid: 4100, startedAt: 17 }),
            "4100-17.json");
        expect("no host process, no list", Places.hostListOf({ host: "work-background" }), "");
        expect("its host lists it", Places.hostSays(owner, list({}, ["s-b", "s-a"])),
            "running");
        expect("its host lists others", Places.hostSays(owner, list({}, ["s-b"])), "gone");
        expect("no list says nothing", Places.hostSays(owner, ""), "unknown");
        expect("another host's list says nothing", Places.hostSays(owner,
            list({ key: "work-attended" }, [])), "unknown");
        expect("another start's list says nothing", Places.hostSays(owner,
            list({ owner: { pid: 4100, startedAt: 18, bootId: "b1", startTicks: 9 } }, [])),
            "unknown");
        const pi = (id, presence) => ({ here: true, address: id.toUpperCase(),
            presence: Object.assign({ session: id }, presence) });
        const hostedHere = pi("s-a", { hosted: { host: "work-background" }, owner });
        const found = (id, running, parked, says) => {
            const at = Places.foundHere(id, running, parked, () => says);
            return [at.as, at.session?.address ?? null];
        };
        expect("one a pi runs here is found running", found("s-a", [pi("s-a")], [], "gone"),
            ["running", "S-A"]);
        expect("one parked here is found parked", found("s-a", [], [pi("s-a")], "gone"),
            ["parked", "S-A"]);
        expect("one elsewhere isn't here", found("s-a",
            [Object.assign(pi("s-a"), { here: false })], [], "gone"), ["", null]);
        expect("one hosted its host lists runs", found("s-a", [hostedHere], [], "running"),
            ["running", "S-A"]);
        expect("one hosted its host doesn't list is stale",
            found("s-a", [hostedHere], [], "gone"), ["stale", "S-A"]);
        expect("one hosted with its host's list unread runs",
            found("s-a", [hostedHere], [], "unknown"), ["running", "S-A"]);
        expect("none", found("s-a", [pi("s-b")], [pi("s-c")], "gone"), ["", null]);

        // A session whose folder is gone starts afresh on its quest, in the
        // folder the harness's quest index says a new one starts in.
        const index = JSON.stringify({ quests: [
            { id: "QEST-REEF", start: "/home/skiff/harbour" },
            { id: "QEST-KEEL", start: "harbour" },
            { id: "QEST-HULL" }] });
        expect("a quest's start", Places.questStart(index, "QEST-REEF"), "/home/skiff/harbour");
        expect("a start not a whole path is none", Places.questStart(index, "QEST-KEEL"), "");
        expect("a quest saying no start has none", Places.questStart(index, "QEST-HULL"), "");
        expect("a quest not held has none", Places.questStart(index, "QEST-MAST"), "");
        expect("no index says none", Places.questStart("", "QEST-REEF"), "");
        let rolled = 0;
        const rolls = [0, 0.5, 0.99];
        const roll = () => rolls[rolled++ % rolls.length];
        const minted = Places.freshId(0x019a1b2c3d4e, roll);
        const version7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
        expect("a fresh id is a UUID version 7", version7.test(minted), true);
        expect("a fresh id begins with when", minted.slice(0, 13), "019a1b2c-3d4e");
        expect("two fresh ids differ", Places.freshId(1, Math.random)
            === Places.freshId(1, Math.random), false);
        const lost = { id: "s-a", cwd: "/gone", quest: "QEST-REEF", title: "Reef" };
        expect("afresh starts anew in its quest's start", Places.afresh(lost, "/home/skiff/harbour",
            "s-new"), { id: "s-new", cwd: "/home/skiff/harbour", quest: "QEST-REEF",
            title: "Reef", fresh: true, afreshFor: "s-a" });
        expect("afresh leaves the one lost as it was", lost.id, "s-a");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            // Qt.exit only asks the engine to stop once this returns.
            Qt.exit(1);
            return;
        }
        console.info("every place holds");
        Qt.exit(0);
    }
}
