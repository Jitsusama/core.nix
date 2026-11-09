// How Machines says every machine of the realms this one links, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `the-launcher-says-every-machine-of-the-realm` check copies
// modules/quickshell/launcher/RealmMachines.js beside this file and runs
// `qml -platform offscreen machines.qml`, over machine records as the
// realms' mycelium nodes serve them. This machine comes first, saying
// so; a row's note says what a machine is in words, never a Mac's
// identifier, and its realm only with two realms; its side says how it
// stands, then what qualifies it; one out of reach dims and says since
// when; the drawer runs from what moves most to the model's identifier,
// never repeating the side, and leaves out what no node said; Start a
// Session says why the node won't place on one, else how it stands; an
// update says how it ended, as its updater said it, or why it couldn't
// be asked. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import QtQuick
import "RealmMachines.js" as Machines

QtObject {
    Component.onCompleted: {
        try {
            run();
            console.log("every machine holds");
            Qt.exit(0);
        } catch (error) {
            console.warn(`FAIL ${error}`);
            Qt.exit(1);
        }
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // The words the vocabulary gives Machines, as words.json says them.
    readonly property var said: ({
        here: "this machine", ready: "ready", busy: "busy", unreachable: "out of reach",
        since: "since {time}", asOf: "as of {time}", short: "short of {what}",
        and: "{first} and {last}", memory: "memory", cpu: "processors",
        battery: "on battery {charge}%", "charging:battery": "{charge}%, on battery",
        "charging:charger": "{charge}%, on the charger", build: "{count} build",
        builds: "{count} builds", vmTest: "{count} VM test", vmTests: "{count} VM tests",
        sameCore: "{commit}, as this machine's", reaches: "reached", unreaches: "not reached",
        freeOf: "{free} free of {total}", cores: "{count} cores", on: "on {realm} \u00b7 {kind}",
        gpu: "graphics", npu: "NPU",
        "updated:done": "{name} updated to {commit}", "updated:done:bare": "{name} updated",
        "updated:unchanged": "{name} was on main already",
        "updated:failed": "{name} didn't update",
        "updated:refused": "{name} refused the update",
        notUpdated: "couldn't update {name}: {why}", unanswered: "its node didn't answer"
    })

    // A part's name, as the vocabulary's machines: parts are named.
    function named(id) {
        return {
            "pressure:cpu": "Processor Pressure", "pressure:memory": "Memory Pressure",
            "pressure:io": "Disk Pressure", memory: "Memory", charge: "Charge", takes: "Takes",
            disk: "Disk", commit: "Commit", core: "Core", step: "Last Update", reboot: "Reboot",
            measured: "Measured", processor: "Processor", accelerators: "Accelerators",
            system: "System", model: "Model", "kind:macos": "Mac", "kind:linux": "Linux",
            "kind:machine": "Machine"
        }[id] ?? id;
    }

    // Two in the morning, 8 Nov 2026, local time.
    readonly property real now: new Date(2026, 10, 8, 2, 0).getTime()

    function place(realms) {
        return {
            said: said, named: named, now: now, realms: realms,
            core: "a97eea75cc7a243aca68875c9c510674a1d7bbdd",
            ago: (at, at2) => `${Math.round((at2 - at) / 60000)} min ago`,
            band: band => (band === "healthy" ? "" : band === "pressure" ? "filling" : band)
        };
    }

    function at(hours, minutes) {
        return new Date(2026, 10, 8, hours, minutes).toISOString();
    }

    // As the nodes serve them: this machine, a Mac short of memory and its
    // processors with nothing to take, a Linux machine on its battery, and
    // one out of reach since 01:20.
    readonly property var records: [
        { name: "skiff", realm: "personal", state: "ready", system: "linux",
            os: "NixOS 26.11 (Zokor)", arch: "x86_64", model: "Microsoft Corporation Surface Laptop 4",
            model_name: "Surface Laptop 4", measured_at: at(1, 58), memory: 8 * 2 ** 30,
            disk: 128 * 2 ** 30, disk_band: "healthy", cpu: { model: "AMD Ryzen 5", cores: 12 },
            accelerators: [{ driver: "amdgpu", kind: "gpu" }],
            power: { on_battery: true, charge: 62 },
            room: { memory_available: 3.2 * 2 ** 30, over: [],
                pressure: { cpu: 0.1, memory: 0, io: 0.12 }, takes: { builds: 1, vm_tests: 1 } },
            reached: { internet: true, office: false },
            updated: { commit: "7307356ee17f4d4afc4ae1ca9b2962a491ae8632", step: "done",
                followed: { core: "a97eea75cc7a243aca68875c9c510674a1d7bbdd" },
                restart: "its kernel changed" } },
        { name: "lantern", realm: "personal", state: "ready", system: "macos",
            os: "macOS 26.6.2", arch: "aarch64", model: "Mac14,2", measured_at: at(1, 59),
            memory: 24 * 2 ** 30, disk: 926 * 2 ** 30, disk_band: "pressure",
            cpu: { model: "Apple M2", cores: 8 }, accelerators: null,
            power: { on_battery: false, charge: 100 },
            room: { memory_available: 8.6 * 2 ** 30, over: ["memory", "cpu"],
                pressure: { cpu: 88.8, memory: 10, io: 0 }, takes: { builds: 0, vm_tests: 0 } },
            updated: { commit: "e1e4aadc4f0172ddb6e8f0cd8f5f36fafc448a4a", step: "done",
                restart: null } },
        { name: "hermit", realm: "personal", state: "unreachable", system: "linux", os: "NixOS",
            model: "Dell Inc. XPS 13", measured_at: at(1, 20), memory: 16 * 2 ** 30,
            room: { memory_available: 2 ** 30, over: [],
                pressure: { cpu: 4, memory: 0, io: 0 }, takes: { builds: 1, vm_tests: 0 } } },
        { name: "anchor", realm: "personal", here: true, state: "ready", system: "linux",
            os: "NixOS 26.11 (Zokor)", model: "Dell Inc. XPS 14 DA14260",
            model_name: "XPS 14 DA14260", measured_at: at(1, 59) },
        // The same machine, said by the work realm's node a minute earlier.
        { name: "anchor", realm: "work", here: true, state: "ready", system: "linux",
            measured_at: at(1, 58) }
    ]

    function run() {
        const one = place(1);
        const listed = Machines.ordered(records);
        const names = listed.map(record => record.name).join(" ");
        expect("this machine first, then those reached, then those out of reach, each once",
            names === "anchor lantern skiff hermit", names);

        const byName = name => listed.find(record => record.name === name);
        const sides = listed.map(record => Machines.side(record, one));
        expect("each side says how it stands first, then what qualifies it",
            JSON.stringify(sides) === JSON.stringify(["this machine",
                "busy \u00b7 short of memory and processors", "ready \u00b7 on battery 62%",
                "out of reach \u00b7 since 01:20"]), JSON.stringify(sides));

        const mac = Machines.note(byName("lantern"), one);
        expect("a Mac with no name in words is a Mac, never its identifier",
            mac === "Mac \u00b7 macOS 26.6.2", mac);
        const surface = Machines.note(byName("skiff"), one);
        expect("a machine's name in words comes first",
            surface === "Surface Laptop 4 \u00b7 NixOS 26.11 (Zokor)", surface);
        const named = Machines.note(Object.assign({}, byName("lantern"),
            { model_name: "MacBook Air" }), place(2));
        expect("the realm is said only with two realms",
            named === "MacBook Air \u00b7 macOS 26.6.2 \u00b7 personal", named);
        const xps = Machines.note(byName("hermit"), one);
        expect("a Linux machine with no name in words is its maker and product",
            xps === "Dell Inc. XPS 13 \u00b7 NixOS", xps);
        const unknown = Machines.note({ name: "buoy", state: "ready" }, one);
        expect("a machine whose system no node said is a machine", unknown === "Machine",
            unknown);

        const drawn = Machines.drawer(byName("skiff"), one);
        expect("the eyebrow says where, then what kind",
            drawn.eyebrow === "on personal \u00b7 Linux" && drawn.title === "skiff",
            `${drawn.eyebrow} ${drawn.title}`);
        const read = drawn.facts.map(fact => `${fact.label}: ${fact.value}`);
        const expected = ["Processor Pressure: 0.1%", "Memory Pressure: 0.0%",
            "Disk Pressure: 0.1%",
            "Memory: 3.2 GiB free of 8 GiB", "Charge: 62%, on battery",
            "Takes: 1 build, 1 VM test", "Disk: 128 GiB", "internet: reached",
            "office: not reached", "Commit: 7307356", "Core: a97eea7, as this machine's",
            "Last Update: done", "Reboot: its kernel changed", "Measured: 2 min ago",
            "Processor: AMD Ryzen 5, 12 cores", "Accelerators: amdgpu graphics",
            "System: NixOS 26.11 (Zokor) \u00b7 x86_64",
            "Model: Microsoft Corporation Surface Laptop 4"];
        expect("the drawer runs from what moves most to the model, never the side again",
            JSON.stringify(read) === JSON.stringify(expected), JSON.stringify(read));

        const strokes = Machines.drawer(byName("lantern"), one).facts
            .filter(fact => fact.figure).map(fact => `${fact.figure.values[0]}`
                + (fact.figure.warn ? "!" : ""));
        expect("each pressure carries its stroke, warning past its queue's limit",
            strokes.join(" ") === "0.888! 0.1! 0", strokes.join(" "));
        const macFacts = Machines.drawer(byName("lantern"), one).facts
            .map(fact => fact.label ?? "");
        expect("a Mac's identifier is last, and nothing unsaid is made up",
            macFacts[macFacts.length - 1] === "Model" && !macFacts.includes("Accelerators")
                && !macFacts.includes("Reboot") && !macFacts.includes("Core"),
            macFacts.join(", "));

        const gone = Machines.drawer(byName("hermit"), one).facts;
        const measured = gone.find(fact => fact.label === "Measured")?.value;
        expect("out of reach, Measured says as of when, and its figures keep their last values",
            measured === "as of 01:20" && gone.some(fact => fact.value === "4.0%"),
            `${measured} ${JSON.stringify(gone)}`);

        // Start a Session's machines, from the same records: the node's
        // placement first, then how each stands.
        const offered = [
            [{ cannot: "out of reach", rather: "" }, byName("hermit")],
            [{ cannot: "", rather: "it's on its battery" }, byName("skiff")],
            [{ cannot: "", rather: "" }, byName("lantern")],
            [{ cannot: "", rather: "" }, byName("anchor")],
            [{ cannot: "", rather: "" }, null]
        ].map(([placed, record]) => Machines.startNote(placed, record, one));
        expect("Start a Session says why the node won't place there, else how it stands",
            JSON.stringify(offered) === JSON.stringify(["out of reach", "it's on its battery",
                "busy \u00b7 short of memory and processors", "this machine", ""]),
            JSON.stringify(offered));

        // How an update ended, from what `mycelium update` printed.
        const ended = [
            '{"Ok":{"Updated":{"step":"done","commit":"c0ffee1234","restarted":[]}}}',
            '{"Ok":{"Updated":{"step":"done"}}}',
            '{"Ok":{"Updated":{"step":"unchanged","commit":"c0ffee1234"}}}',
            '{"Ok":{"Updated":{"step":"failed"}}}',
            '{"Ok":{"Updated":{"step":"refused"}}}',
            '{"Ok":{"Updated":{"step":"switching"}}}',
            '{"Err":"skiff is out of reach"}',
            ""
        ].map(text => Machines.updateEnded("skiff", text, said));
        expect("an update says how it ended, its commit short, and why one couldn't be asked",
            JSON.stringify(ended) === JSON.stringify(["skiff updated to c0ffee1",
                "skiff updated", "skiff was on main already", "skiff didn't update",
                "skiff refused the update", "couldn't update skiff: its node didn't answer",
                "couldn't update skiff: skiff is out of reach",
                "couldn't update skiff: its node didn't answer"]), JSON.stringify(ended));
    }
}
