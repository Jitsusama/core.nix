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
            { name: "playing", value: "48 kHz, 32 bit, period 1024 (21.3 ms)" }] };
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
    }
}
