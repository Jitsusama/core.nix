// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml)
// through Quickshell itself, where it decides, over stand-in devices as
// the sound server would give them, with what desk-facts would say of
// their jacks and niri's workspaces on the machine's own panel alone: a
// jack with nothing plugged into it isn't listed, and says so where it's
// shown; one with headphones in is; a card starting or stopping to play
// lists nothing again, since only its jack says whether it's listed;
// speakers desk-facts says no jack of are; a display's output waits for a display; and a headset's microphone
// isn't listened to while the headset plays music, since listening would
// switch it to calls, though it is once it's in a call. The sandbox has no
// sound server, so which device is in use, and which one is watched, can't
// be stood in for here. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher
import "launcher/Sounds.js" as Sounds

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Sound {
        id: sound
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

    // The machine's own card, its headphones' jack its device 0, and an
    // HDMI output on the same card that desk-facts says nothing of.
    readonly property var headphones: device("alsa_output.pci.headphones", true,
        { "alsa.id": "PCH", "alsa.device": "0" })
    readonly property var speakers: device("alsa_output.pci.speakers", true,
        { "alsa.id": "PCH", "alsa.device": "1" })
    readonly property var hdmi: device("alsa_output.pci.hdmi-stereo", true,
        { "alsa.id": "PCH", "alsa.device": "3" })
    // WirePlumber's stand-in for a headset's microphone, carrying the
    // headset's profile, as the headset node beside it would say.
    function microphone(profile) {
        return device("bluez_input.AA:BB:CC:DD:EE:FF", false,
            { "bluez5.loopback": "true", "api.bluez5.profile": profile });
    }

    function jack(plugged) {
        return { "PCH,0:out": [{ name: "Plugged in", value: plugged }] };
    }

    // The same jack, its card now playing, as desk-facts says once sound
    // starts through it.
    function playingThrough(plugged) {
        return { "PCH,0:out": [{ name: "Plugged in", value: plugged },
            { name: "playing", value: "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)" }] };
    }

    // Each time whether the headphones are listed is worked out again, as
    // Sound's devices are: a new object each time, so each time is heard.
    property int listings: 0
    readonly property var headphonesListed: ({ listed: sound.listed(headphones) })
    onHeadphonesListedChanged: listings += 1

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

    function run() {
        sound.facts = jack("nothing");
        expect("an empty jack isn't listed", !sound.listed(headphones), "listed");
        expect("an empty jack says so", sound.made(headphones).note
            .includes(sound.said("unplugged")), sound.made(headphones).note);
        expect("speakers with no jack are listed", sound.listed(speakers), "not listed");
        expect("a display's output waits for a display", !sound.listed(hdmi), "listed");
        sound.facts = jack("headphones");
        expect("a jack with headphones in is listed", sound.listed(headphones), "not listed");
        expect("and says nothing of being empty", !sound.made(headphones).note
            .includes(sound.said("unplugged")), sound.made(headphones).note);
        const before = listings;
        sound.facts = playingThrough("headphones");
        sound.facts = jack("headphones");
        expect("a card starting and stopping lists nothing again", listings === before,
            `listed ${listings - before} more times`);
        sound.facts = playingThrough("nothing");
        expect("its jack emptying does", listings > before && !sound.listed(headphones),
            `listed ${listings - before} more times`);

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
    }

    // A device's drawer says where it is and which way it goes, then each
    // of its parts the step holds, what the kernel says, and what the sound
    // server knows it by last; never whether it's muted, its row's, nor
    // its Level, which reading would open a stream on it.
    function drawers() {
        const card = device("alsa_output.pci.speakers", true, { "alsa.id": "PCH",
            "alsa.device": "1", "device.bus": "pci", "api.alsa.path": "hw:PCH,1",
            "priority.session": "1000", "alsa.resolution_bits": "24" });
        card.id = 52;
        sound.facts = { "PCH,1:out": [{ name: "playing", value: "48 kHz \u00b7 32 bit" }] };
        const plate = sound.drawerOf(card);
        expect("a built-in output says so over its name",
            plate.eyebrow === "built in \u00b7 output", plate.eyebrow);
        const said = plate.facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        const labels = plate.facts.map(fact => fact.label).join(", ");
        // The kernel's fact is named by the launcher's vocabulary, which
        // there's none of here, so by its own name.
        expect("its parts, the kernel's, then what it's known by",
            labels === "Volume, Resolution, Playing, Node, ALSA Device, Priority", said);
        expect("its volume says its decibels after a dot",
            plate.facts[0].value === `50% \u00b7 ${Sounds.decibels(0.5)}`, plate.facts[0].value);
        expect("and draws its share as a stroke", plate.facts[0].figure?.shape === "stroke"
            && plate.facts[0].figure.values[0] === 0.5, JSON.stringify(plate.facts[0].figure));
        expect("what it's known by is as the server says it",
            said.endsWith("Node=52; ALSA Device=hw:PCH,1; Priority=1000"), said);
        card.audio.muted = true;
        expect("a muted one doesn't say so again, nor its level", !sound.drawerOf(card).facts
            .some(fact => fact.label === "Muted" || fact.label === "Level"), said);
        expect("a headset's microphone is over Bluetooth",
            sound.drawerOf(calling()).eyebrow === "Bluetooth \u00b7 input",
            sound.drawerOf(calling()).eyebrow);
    }

    function calling() {
        return microphone("headset-head-unit");
    }
}
