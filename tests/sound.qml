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

        programs();
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
}
