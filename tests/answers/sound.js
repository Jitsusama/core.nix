// The cases of sound for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Sounds.js" as Sounds

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // What Sound makes of what a device and a stream say of themselves.
    machine("each program's streams together", Sounds.grouped(["a1", "b1", "a2"],
        stream => stream.charAt(0)), [{ name: "a", streams: ["a1", "a2"] },
        { name: "b", streams: ["b1"] }]);
    machine("a step louder", Sounds.stepped(0.42, 0.05, 0.05), 0.45);
    machine("never past the whole", Sounds.stepped(0.98, 0.05, 0.05), 1);
    machine("never below nothing", Sounds.stepped(0.02, -0.05, 0.05), 0);
    machine("a level in decibels", [Sounds.decibels(1), Sounds.decibels(0.5),
        Sounds.decibels(0.35), Sounds.decibels(1.2), Sounds.decibels(0)],
        ["0 dB", "-18.1 dB", "-27.4 dB", "+4.8 dB", "-\u221e dB"]);
    // The machine's own are called what they are, by their port, and a
    // headset's microphone as its; any other keeps its own name.
    const soundOut = "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__";
    const soundIn = "alsa_input.pci-0000_00_1f.3-platform-sof_sdw.HiFi__";
    machine("what a device is called", [`${soundOut}Speaker__sink`, `${soundIn}Mic__source`,
        `${soundOut}Headphones__sink`, `${soundIn}Headset__source`,
        "bluez_input.00:11:22:33:44:55", "bluez_output.00_11_22_33_44_55.1",
        `${soundOut}HDMI1__sink`, undefined].map(name => Sounds.called(name)),
        ["speakers", "microphones", "jack", "jack-microphone", "its-microphone", "", "", ""]);
    // A USB microphone named as its own headphone socket is, its
    // device's microphone; one named alone keeps its name.
    const usb = "alsa_input.usb-Shure_Inc_Shure_MV7-00.mono-fallback";
    machine("a microphone named as an output", [Sounds.called(usb, true),
        Sounds.called(usb, false)], ["its-microphone", ""]);

    // What Sound says of Bluetooth headphones, as the sound server has
    // them while they play music in AAC: their output, and WirePlumber's
    // stand-in for their microphone, which names them only by address.
    const soundHeadphones = { "api.bluez5.address": "12:34:56:78:9A:BC",
        "api.bluez5.codec": "aac", "api.bluez5.profile": "a2dp-sink",
        "node.description": "Muse", "device.id": 65, "card.profile.device": 1 };
    const soundStandIn = { "bluez5.loopback": "true", "node.description": "Muse" };
    machine("a Bluetooth output's address", Sounds.address("bluez_output.12_34_56_78_9A_BC.1",
        soundHeadphones), "12:34:56:78:9A:BC");
    machine("its microphone's stand-in's, from its name",
        Sounds.address("bluez_input.12:34:56:78:9A:BC", soundStandIn), "12:34:56:78:9A:BC");
    machine("a card's own output has none", Sounds.address("alsa_output.pci", {}), "");
    machine("the stand-in is one", [Sounds.standIn(soundStandIn),
        Sounds.standIn(soundHeadphones)], [true, false]);
    machine("music, not a2dp", Sounds.carries(soundHeadphones["api.bluez5.profile"]),
        "music");
    machine("calls, not hfp", [Sounds.carries("headset-head-unit"),
        Sounds.carries("hfp-hf")], ["calls", "calls"]);
    machine("no profile, nothing carried", Sounds.carries(undefined), "");
    return asked;
}
