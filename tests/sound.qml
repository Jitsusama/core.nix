// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml)
// through Quickshell itself, where it still makes a device's parts and
// drawer, over stand-in devices as the sound server would give them: a
// headset's microphone isn't listened to while the headset plays music,
// since listening would switch it to calls, though it is once it's in a
// call; a drawer says what the device is doing; and a device's parts are
// kept while the launcher's shut and nothing they show moved. Which
// devices are listed, and in what order, is the sound daemon's
// (modules/sound's rows.rs), and Sound's rows as it serves them are held
// in tests/sound-served.qml. The sandbox has no sound server, so which
// device is in use, and which one is watched, can't be stood in for here.
// It exits 0 once every case holds, and otherwise 1, saying which case
// didn't.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
import "launcher" as Launcher
import "launcher/Sounds.js" as Sounds
import "launcher/Targets.js" as Targets

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Sound {
        id: sound
    }

    // A device as the sound server holds one, an object as its nodes are.
    component Node: QtObject {
        property string name
        property string nickname: ""
        property string description: name
        property bool isSink: true
        property bool isStream: false
        property int type: isSink ? PwNodeType.AudioSink : PwNodeType.AudioSource
        property var properties: ({ "device.bus": "usb" })
        property var audio: ({ volume: 0.5, muted: false })
    }

    Node {
        id: deskSpeakers
        name: "usb-desk-speakers"
    }

    // The launcher as Sound asks it, shut, and whether Sound's rows are
    // wanted (Provider.qml's wanted).
    QtObject {
        id: shutLauncher

        property bool open: false
        property bool wanting: true
        readonly property var held: []

        function wants(source) {
            return wanting;
        }
    }

    Launcher.Sound {
        id: shutSound
        launcher: shutLauncher
        serverNodes: [deskSpeakers]
    }

    // A device as the sound server gives one: its name, which way it goes,
    // its properties and its audio.
    function device(name, sink, properties) {
        return {
            name: name,
            nickname: "",
            description: name,
            isSink: sink,
            properties: properties,
            audio: { volume: 0.5, muted: false }
        };
    }

    // The machine's own card, its headphones' jack its device 0.
    readonly property var headphones: device("alsa_output.pci.headphones", true,
        { "alsa.id": "PCH", "alsa.device": "0" })
    readonly property var speakers: device("alsa_output.pci.speakers", true,
        { "alsa.id": "PCH", "alsa.device": "1" })
    // WirePlumber's stand-in for a headset's microphone, carrying the
    // headset's profile, as the headset node beside it would say.
    function microphone(profile) {
        return device("bluez_input.AA:BB:CC:DD:EE:FF", false,
            { "bluez5.loopback": "true", "api.bluez5.profile": profile });
    }

    // Quickshell hears Qt.exit only once the shell has loaded, so the cases
    // run just after.
    Component.onCompleted: Qt.callLater(root.cases)

    function cases() {
        let status = 0;
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            status = 1;
        }
        Qt.callLater(() => Qt.exit(status));
    }

    // A row made again with getters (Provider.qml's gotten) keeps every
    // field it had, each getter read as it's asked, through rows of
    // shapes grown one from another, where turning a field into a getter
    // in place lost the field grown after it under Qt 6.12 (the shell's
    // crash of 2026-11-11). A program's row is made that way, its options
    // read as :setl asks (tests/sound-served.qml).
    function gotten() {
        let muted = true;
        const grown = sound.thing({ key: "grown", name: "Grown", hue: "action" });
        const first = sound.gotten(grown, { side: () => muted ? "yes" : "no" });
        grown.sideGrown = "kept";
        const second = sound.gotten(grown, { side: () => muted ? "yes" : "no" });
        expect("a row made again keeps the field it was given since",
            Object.assign({}, second).sideGrown === "kept", Object.assign({}, second).sideGrown);
        expect("and every field it had", first.key === "grown" && first.name === "Grown"
            && second.hue === "action" && first.chord.length === 0, JSON.stringify(first));
        muted = false;
        expect("its getter is read as it's asked", second.side === "no", second.side);
    }

    function run() {
        const playing = microphone("a2dp-sink");
        expect("a headset playing music would switch", sound.switches(playing), "it wouldn't");
        const unheard = sound.levelPart(playing);
        expect("so its Level isn't listened to", unheard.side === sound.said("unheard")
            && unheard.note === sound.said("level-switches"), `${unheard.side}, ${unheard.note}`);
        const calling = microphone("headset-head-unit");
        expect("a headset in a call wouldn't", !sound.switches(calling), "it would");
        const heard = sound.levelPart(calling);
        expect("so its Level is listened to", heard.side === sound.said("silent")
            && heard.note === sound.said("level-in"), `${heard.side}, ${heard.note}`);

        drawers();
        gotten();
        partsKept();
    }

    // Shut, a device's parts asked for again with nothing moved, as its
    // spots are a minute after each closing (Launcher.qml's
    // spotsFreshened), make none of them again; nor does its beginning to
    // run, which gives its node a period and a latency no part says; and
    // shown, they're made as they're asked for. What its card says is
    // served (tests/sound-served.qml), so it makes nothing here.
    function partsKept() {
        shutLauncher.wanting = false;
        const properties = deskSpeakers.properties;
        const card = { "device.bus": "usb", "alsa.id": "Desk", "alsa.device": "0" };
        deskSpeakers.properties = card;
        const first = shutSound.verbs(deskSpeakers);
        const before = shutSound.tally.rows;
        const again = shutSound.verbs(deskSpeakers);
        expect("shut, with nothing moved, its parts are made no more",
            shutSound.tally.rows === before && again === first && first.length > 0,
            `${shutSound.tally.rows - before} made of ${first.length}`);
        deskSpeakers.properties = Object.assign({}, card, { "api.alsa.period-size": "512",
            "node.max-latency": "16384/48000" });
        const running = shutSound.verbs(deskSpeakers);
        expect("nor as it begins to run", shutSound.tally.rows === before && running === first,
            `${shutSound.tally.rows - before} made of ${first.length}`);
        deskSpeakers.properties = properties;
        shutLauncher.wanting = true;
        const shown = shutSound.tally.rows;
        shutSound.verbs(deskSpeakers);
        expect("shown, they're made as they're asked for", shutSound.tally.rows > shown,
            `${shutSound.tally.rows - shown} made`);
    }

    // A device's drawer says where it is and which way it goes, then what
    // pipewire says it's doing, each of its parts the step holds, what the
    // kernel says, the format the graph moves where the card doesn't say
    // the same, and what the sound server knows it by last; never whether
    // it's muted, its row's, nor its Level, which reading would open a
    // stream on it.
    function drawers() {
        const card = device("alsa_output.pci.speakers", true, { "alsa.id": "PCH",
            "alsa.device": "1", "device.bus": "pci", "api.alsa.path": "hw:PCH,1",
            "priority.session": "1000" });
        card.id = 52;
        card.state = "running";
        card.audio.format = { rate: 48000, channels: 2, sample: "S32LE" };
        const plate = sound.drawerOf(card);
        expect("a built-in output says so over its name, as it's written",
            plate.eyebrow === "built in \u00b7 output" && plate.eyebrowNames !== true,
            plate.eyebrow);
        const made = device("stand-in-speakers", true, {});
        expect("one the machine makes is on this machine",
            sound.drawerOf(made).eyebrow === "on this machine \u00b7 output",
            sound.drawerOf(made).eyebrow);
        const said = plate.facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        const labels = plate.facts.map(fact => fact.label).join(", ");
        // Its Hardware, what the kernel says of its card and its Volume
        // are served, so tests/sound-served.qml says them; with none, the
        // format the graph moves is its Format.
        expect("its state, its format, then what it's known by",
            labels === "State, Format, Priority, ALSA Device, Graph ID", said);
        expect("what identifies it is marked so, its Graph ID last", plate.facts
            .filter(fact => fact.identifies).map(fact => fact.label).join(", ")
            === "ALSA Device, Graph ID", said);
        expect("its state is pipewire's word", plate.facts[0].value === "running", said);
        const levels = [Sounds.channelLevels(["Front Left", "Front Right"], ["100%", "100%"], "even"),
            Sounds.channelLevels(["Front Left", "Front Right"], ["100%", "80%"], "even"),
            Sounds.channelLevels(["Front Left", "Front Right", "Rear Left"], ["9%", "8%", "7%"],
                "even"), Sounds.channelLevels([], [], "even")].map(each => each.join(", "));
        expect("channels at one level are balanced, a pair's otherwise named without front",
            levels.join("; ") === "even; left 100%, right 80%; "
            + "front left 9%, front right 8%, rear left 7%; ", levels.join("; "));
        const formats = node => sound.drawerOf(node).facts
            .filter(fact => fact.label.endsWith("Format"))
            .map(fact => `${fact.label}=${fact.value}`).join("; ");
        card.audio.format = { rate: 48000, channels: 2, sample: "F32LE" };
        expect("one the card doesn't say is its Format",
            formats(card) === "Format=48 kHz \u00b7 32 bit float", formats(card));
        card.state = "suspended";
        card.audio.format = {};
        expect("a suspended one says so, and no format", formats(card) === ""
            && sound.drawerOf(card).facts[0].value === "suspended", formats(card));
        expect("what it's known by is as the server says it",
            said.endsWith("Priority=1000; ALSA Device=hw:PCH,1; Graph ID=52"), said);
        card.audio.muted = true;
        expect("a muted one doesn't say so again, nor its level", !sound.drawerOf(card).facts
            .some(fact => fact.label === "Muted" || fact.label === "Level"), said);
        // How it keeps time, as the sound daemon serves it: ahead of its
        // Format, its Dropouts warned only past none.
        card.state = "running";
        card.audio.format = { rate: 48000, channels: 2, sample: "F32LE" };
        sound.timings = { 52: { id: 52, quantum: 1024, rate: 48000, ms: 21.333, busy: 0.0312,
            errors: 0 } };
        const keeping = node => sound.drawerOf(node).facts
            .filter(fact => ["Dropouts", "Load", "Cycle", "Format"].includes(fact.label));
        expect("its timing, ahead of its format", keeping(card)
            .map(fact => `${fact.label}=${fact.value}`).join("; ") === "Dropouts=none; "
            + "Load=3.12% of each cycle; Cycle=1,024 frames \u00b7 21.3 ms; "
            + "Format=48 kHz \u00b7 32 bit float", JSON.stringify(keeping(card)));
        expect("its load a stroke, its dropouts calm at none", keeping(card)[1].figure?.values[0]
            === 0.0312 && keeping(card)[0].warns === false, JSON.stringify(keeping(card)));
        sound.timings = { 52: Object.assign({}, sound.timings[52], { errors: 2 }) };
        expect("a dropout warns", keeping(card)[0].value === "2" && keeping(card)[0].warns,
            JSON.stringify(keeping(card)[0]));
        const dropouts = errors => {
            sound.timings = { 52: Object.assign({}, sound.timings[52], { errors }) };
            return keeping(card)[0].value;
        };
        expect("dropouts counted whole", [5, 12, 1500].map(dropouts).join("; ")
            === "5; 12; 1,500", [5, 12, 1500].map(dropouts).join("; "));
        sound.timings = { 52: Object.assign({}, sound.timings[52], { busy: 0.000938 }) };
        expect("a light load keeps its three significant digits",
            keeping(card)[1].value === "0.0938% of each cycle", JSON.stringify(keeping(card)[1]));
        // A measure keeps its zeros, as every measure does (VKCF3Z,
        // 2026-11-08): 960 frames at 48 kHz take 20.0 ms.
        sound.timings = { 52: Object.assign({}, sound.timings[52],
            { busy: 0.031, quantum: 960, ms: 20 }) };
        const zeros = keeping(card).slice(1, 3).map(fact => fact.value).join("; ");
        expect("a load and a cycle keep their zeros",
            zeros === "3.10% of each cycle; 960 frames \u00b7 20.0 ms", zeros);
        sound.timings = { 52: Object.assign({}, sound.timings[52],
            { busy: null, unfinished: true }) };
        expect("one that ran past its cycle is over all of it, its stroke full, warned",
            keeping(card)[1].value === "over 100% of each cycle" && keeping(card)[1].warns
            && keeping(card)[1].figure?.values[0] === 1 && keeping(card)[1].figure?.warn,
            JSON.stringify(keeping(card)[1]));
        // The daemon's records, held by key, are its timings by node.
        const held = sound.timingsOf({
            "timing:52": { key: "timing:52", kind: "timing", id: 52, quantum: 1024 },
            "stream:90": { key: "stream:90", kind: "stream", id: 90 }
        });
        expect("each node's timing by its id, and nothing else",
            JSON.stringify(Object.keys(held)) === "[\"52\"]" && held[52].quantum === 1024,
            JSON.stringify(held));
        sound.timings = {};
        expect("none while the daemon doesn't say it runs", keeping(card).length === 1,
            JSON.stringify(keeping(card)));
        expect("a headset's microphone is over Bluetooth",
            sound.drawerOf(calling()).eyebrow === "Bluetooth \u00b7 input",
            sound.drawerOf(calling()).eyebrow);
        programs();
    }

    // A program with no desktop entry is named as it names itself, for
    // Now's open-programs list; the sound daemon serves a program's own
    // drawer (modules/sound's drawer.rs). How nodes keep time is watched
    // for a device's drawer and its programs', never its list's.
    function programs() {
        expect("a program with no entry is named as it names itself",
            sound.programOf({ name: "pw-cat", properties: { "application.name": "pw-cat" } })
                .name === "pw-cat", "capitalised");
        const among = [speakers, headphones];
        for (const [key, timed] of [["sound:alsa_output.pci.speakers", speakers.name],
            ["sound:alsa_output.pci.speakers:program:Firefox", speakers.name],
            ["sound:alsa_output.pci.speakers:programs", ""], ["sound:elsewhere", ""]])
            expect(`${key} times ${timed || "nothing"}`, sound.timedBy(key, among) === timed,
                sound.timedBy(key, among));
        targets();
    }

    // Where a program's sound goes: the one target the sound daemon says a
    // program's streams share; the default chosen where they're sent to no
    // device listed; and which streams a send moves.
    function targets() {
        const sent = { 90: "stand-in-headphones" };
        const ninety = { id: 90 };
        expect("a program's streams sent alike are sent there",
            Targets.sentTo([ninety, ninety], sent) === "stand-in-headphones"
            && Targets.sentTo([ninety, { id: 93 }], sent) === ""
            && Targets.sentTo([], sent) === "", "they weren't");
        const out = { name: "stand-in-headphones", properties: { "object.serial": "61" } };
        const chosen = choices => choices.map(each => `${each.device?.name ?? "default"}`
            + `${each.chosen ? "*" : ""}`).join(" ");
        expect("the default leads, chosen while it's sent nowhere listed",
            chosen(Targets.choices([speakers, out], "")) === "default* "
            + "alsa_output.pci.speakers stand-in-headphones"
            && chosen(Targets.choices([speakers], "stand-in-headphones")) === "default* "
            + "alsa_output.pci.speakers", chosen(Targets.choices([speakers, out], "")));
        expect("a device sent to is chosen, by its name or its serial",
            chosen(Targets.choices([speakers, out], "61")) === "default "
            + "alsa_output.pci.speakers stand-in-headphones*",
            chosen(Targets.choices([speakers, out], "61")));
        const ids = Targets.sendable([{ id: "90 -d" }, { id: 9.5 }, { id: -1 }, {}, ninety]);
        expect("only a stream with a whole id is sent", JSON.stringify(ids) === '["90"]',
            JSON.stringify(ids));
    }

    function calling() {
        return microphone("headset-head-unit");
    }
}
