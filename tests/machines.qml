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
        battery: "on battery {charge}%", "charging:battery": "{charge}% \u00b7 on battery",
        "charging:charger": "{charge}% \u00b7 on the charger", build: "{count} build",
        builds: "{count} builds", vmTest: "{count} VM test", vmTests: "{count} VM tests",
        sameCore: "{commit}, as this machine's", reaches: "reached", unreaches: "not reached",
        freeOf: "{free} free of {total}", sized: "{total} in all", cores: "{count} cores",
        on: "on {realm} \u00b7 machine",
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
            // When.js's day, which its own case holds: nothing today, else
            // the day before.
            day: (moment, today) => (new Date(moment).getDate() === new Date(today).getDate()
                ? "" : "yesterday"),
            clock: moment => Qt.formatDateTime(new Date(moment), "HH:mm"),
            band: band => (band === "healthy" ? "" : band.replace(/-/g, " "))
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
            disk: 4.08e9, disk_free: 1.07e9, disk_band: "healthy",
            cpu: { model: "AMD Ryzen(TM) 5 4680U", cores: 12 },
            accelerators: [{ driver: "amdgpu", kind: "gpu" }],
            power: { on_battery: true, charge: 62 },
            room: { memory_available: 3.2 * 2 ** 30, over: [],
                pressure: { cpu: 0.1, memory: 0, io: 0.12 }, takes: { builds: 1, vm_tests: 1 } },
            reached: { internet: true, office: false },
            updated: { commit: "7307356ee17f4d4afc4ae1ca9b2962a491ae8632", step: "done",
                at: new Date(2026, 10, 7, 23, 40).toISOString(),
                followed: { core: "a97eea75cc7a243aca68875c9c510674a1d7bbdd" },
                restart: "its kernel changed" } },
        { name: "lantern", realm: "personal", state: "ready", system: "macos",
            os: "macOS 26.6.2", arch: "aarch64", model: "Mac14,2", measured_at: at(1, 59),
            memory: 24 * 2 ** 30, disk: 926 * 2 ** 30, disk_band: "filling",
            cpu: { model: "Apple M2", cores: 8 }, accelerators: null,
            power: { on_battery: false, charge: 100 },
            room: { memory_available: 8.6 * 2 ** 30, over: ["memory", "cpu"],
                pressure: { cpu: 88.8, memory: 10, io: 0 }, takes: { builds: 0, vm_tests: 0 } },
            updated: { commit: "e1e4aadc4f0172ddb6e8f0cd8f5f36fafc448a4a", step: "failed",
                at: at(1, 30), restart: null } },
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

        const here = Machines.hereItems(byName("anchor"), one);
        expect("this machine's side leads with what tells it apart, its system giving way first",
            JSON.stringify(here) === JSON.stringify([{ said: "this machine", keep: 3 },
                { said: "XPS 14 DA14260", keep: 2 }, { said: "NixOS 26.11 (Zokor)", keep: 1 }]),
            JSON.stringify(here));
        const hereDrawn = Machines.drawer(byName("anchor"), one);
        const hereFacts = hereDrawn.facts.map(fact => `${fact.label}: ${fact.value}`);
        expect("this machine's drawer is every machine's",
            hereDrawn.eyebrow === "on personal \u00b7 machine" && JSON.stringify(hereFacts)
                === JSON.stringify(["Measured: 01:59", "System: NixOS 26.11 (Zokor)",
                    "Model: Dell Inc. XPS 14 DA14260"]),
            `${hereDrawn.eyebrow} ${JSON.stringify(hereFacts)}`);

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
        expect("the eyebrow says where, then what kind of thing, never its system",
            drawn.eyebrow === "on personal \u00b7 machine" && drawn.title === "skiff",
            `${drawn.eyebrow} ${drawn.title}`);
        const read = drawn.facts.map(fact => `${fact.label}: ${fact.value}`);
        const expected = ["Processor Pressure: 0.1%", "Memory Pressure: 0.0%",
            "Disk Pressure: 0.1%",
            "Memory: 3.44 GB free of 8.59 GB", "Charge: 62% \u00b7 on battery",
            "Takes: 1 build, 1 VM test", "Disk: 1.07 GB free of 4.08 GB", "internet: reached",
            "office: not reached", "Commit: 7307356", "Core: a97eea7, as this machine's",
            "Last Update: done \u00b7 yesterday 23:40", "Reboot: its kernel changed",
            "Measured: 01:58",
            "Processor: AMD Ryzen 5 4680U \u00b7 12 cores", "Accelerators: amdgpu graphics",
            "System: NixOS 26.11 (Zokor) \u00b7 x86_64",
            "Model: Microsoft Corporation Surface Laptop 4"];
        expect("the drawer runs from what moves most to the model, never the side again",
            JSON.stringify(read) === JSON.stringify(expected), JSON.stringify(read));
        const marked = drawn.facts.filter(fact => fact.mark)
            .map(fact => `${fact.label} ${fact.mark}`);
        expect("Last Update alone wears a mark, the update's",
            JSON.stringify(marked) === JSON.stringify(["Last Update machines:step"]),
            JSON.stringify(marked));

        const strokes = Machines.drawer(byName("lantern"), one).facts
            .filter(fact => fact.figure).map(fact => `${fact.figure.values[0]}`
                + (fact.figure.warn ? "!" : ""));
        expect("each pressure carries its stroke, warning past its queue's limit",
            strokes.join(" ") === "0.888! 0.1! 0", strokes.join(" "));
        const macDrawn = Machines.drawer(byName("lantern"), one).facts;
        const lastUpdate = macDrawn.find(fact => fact.label === "Last Update")?.value;
        expect("an update ended today says only its time",
            lastUpdate === "failed \u00b7 01:30", lastUpdate);
        const unsaid = Machines.drawer(Object.assign({}, byName("lantern"), {
            updated: { step: "done" } }), one).facts
            .find(fact => fact.label === "Last Update")?.value;
        expect("one whose node says no time says how it ended alone", unsaid === "done",
            unsaid);
        const macDisk = macDrawn.find(fact => fact.label === "Disk")?.value;
        expect("a disk whose node says nothing free says its size is all it is, then its band",
            macDisk === "994 GB in all \u00b7 filling", macDisk);
        const xeon = Machines.processor("Intel(R) Xeon(R) CPU @ 2.80GHz");
        expect("a processor drops what's typed for lawyers, and spaces its clock's unit",
            xeon === "Intel Xeon @ 2.80 GHz", xeon);
        const macFacts = macDrawn.map(fact => fact.label ?? "");
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

        // How an update ended, from its node's answer to the ask.
        const ended = [
            { id: "find-1", updated: { step: "done", commit: "c0ffee1234", restarted: [] } },
            { id: "find-1", updated: { step: "done" } },
            { id: "find-1", updated: { step: "unchanged", commit: "c0ffee1234" } },
            { id: "find-1", updated: { step: "failed" } },
            { id: "find-1", updated: { step: "refused" } },
            { id: "find-1", updated: { step: "switching" } },
            { id: "find-1", error: { code: "refused", message: "skiff is out of reach" } },
            null
        ].map(answer => Machines.updateEnded("skiff", answer, said));
        expect("an update says how it ended, its commit short, and why one couldn't be asked",
            JSON.stringify(ended) === JSON.stringify(["skiff updated to c0ffee1",
                "skiff updated", "skiff was on main already", "skiff didn't update",
                "skiff refused the update", "couldn't update skiff: its node didn't answer",
                "couldn't update skiff: skiff is out of reach",
                "couldn't update skiff: its node didn't answer"]), JSON.stringify(ended));
    }
}
