// The cases of accessories for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Links.js" as Links

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    const machine = Common.checker(failures, () => asked++);
    // How each Bluetooth link is doing, from what bluetooth-links writes.
    const btMap = `${"1".repeat(22)}${"0".repeat(18)}${"1".repeat(39)}`;
    const btClassic = { address: "00:1B:66:AA:BB:CC", rssi: -6, quality: 230, power: 4,
        channels: 61, map: btMap, le: false };
    const btFile = JSON.stringify({ at: 1, links: [btClassic] });
    machine("the links a reader writes", Links.links(btFile).length, 1);
    machine("no links connected", Links.links(JSON.stringify({ links: [] })), []);
    machine("no reader", Links.links(""), null);
    machine("a file that isn't the reader's", Links.links("{}"), null);
    machine("a link by its address", Links.of(Links.links(btFile), btClassic.address).rssi, -6);
    machine("a device with no link", Links.of(Links.links(btFile), "C3:9C:E3:2A:F3:6E"), null);
    machine("a classic link below its best range", Links.signal(btClassic),
        { side: "-6 dB", value: 0.85,
            note: "below the range its radio hears best, sending at 4 dBm" });
    machine("a classic link in its best range", Links.signal({ rssi: 0, le: false }),
        { side: "in range", value: 1, note: "where its radio hears best" });
    machine("a classic link above its best range", Links.signal({ rssi: 3, le: false }).side,
        "+3 dB");
    machine("a classic link far below", Links.signal({ rssi: -60, le: false }).value, 0);
    machine("a Low Energy link's signal", Links.signal({ rssi: -58, power: null, le: true }),
        { side: "-58 dBm", value: 0.6, note: "" });
    machine("a signal not said", Links.signal({ rssi: null, le: true }), null);
    machine("no link, no signal", Links.signal(null), null);
    machine("the link's quality", Links.quality(btClassic).side, "90%");
    machine("a perfect link", Links.quality({ quality: 255 }).value, 1);
    machine("a quality not said", Links.quality({ quality: null }), null);
    const btChannels = Links.channels(btClassic);
    machine("the channels clear", [btChannels.side, btChannels.clear, btChannels.note],
        ["61 of 79 clear", 61, "the link skips the other 18 as noisy"]);
    machine("the channels as a strip", btChannels.values, [1, 1, 1, 1, 1, 0.5, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    machine("every channel clear", Links.channels({ map: "1".repeat(79) }).note,
        "the link skips none as noisy");
    machine("no channel map", Links.channels({ map: null }), null);
    machine("music's codec", Links.codec("aac"), { name: "AAC", note: "" });
    machine("aptX in its own case", Links.codec("aptx_hd").name, "aptX HD");
    machine("a call's codec", Links.codec("msbc"),
        { name: "mSBC", note: "call quality, while its microphone is in use" });
    machine("any Opus", Links.codec("opus_05_51").name, "Opus");
    machine("a codec not known", Links.codec("new_one").name, "NEW_ONE");
    machine("no codec", Links.codec(undefined), null);
    machine("what music's codec carries", Links.carried({ "api.bluez5.profile": "a2dp-sink",
        "node.rate": "1/44100" }, 2), { profile: "music", rate: "44.1 kHz",
        channels: "stereo" });
    machine("what a call's codec carries", Links.carried({
        "api.bluez5.profile": "headset-head-unit", "audio.rate": 16000,
        "audio.channels": 1 }, 0), { profile: "calls", rate: "16 kHz", channels: "mono" });
    machine("six channels", Links.carried({ "audio.channels": 6 }, 0).channels, "6 channels");
    machine("a node saying nothing", Links.carried(null, 0),
        { profile: "", rate: "", channels: "" });
    machine("an old reader's link has its one read", Links.signals(btClassic), [0.85]);
    machine("a link held unencrypted", Links.held({ le: true, held: { central: false,
        outgoing: true, security: "authenticated" } }), {
        side: "Low Energy, peripheral, not encrypted",
        note: "this machine started it, though the device proved who it is" });
    machine("an old reader's link says nothing of how", Links.held(btClassic), null);
    machine("the version of Bluetooth", Links.version(14), "Bluetooth 6.0");
    machine("a version not known", Links.version(99), "");
    machine("a chip's maker", [Links.maker(2), Links.maker(9999)], ["Intel", ""]);
    machine("an adapter by its name", Links.adapter(Links.adapters(JSON.stringify({
        adapters: [{ name: "hci0" }], links: [] })), "hci0").name, "hci0");
    machine("an old reader's file has no adapters", Links.adapters(btFile), []);
    machine("an adapter read once has no traffic", Links.traffic({ received: null,
        sent: null, traffic: [] }), null);
    machine("a busy adapter's traffic", Links.traffic({ received: 2.5e6, sent: 0,
        traffic: [2.5e6] }), { side: "20 Mb/s", values: [1],
        note: "20 Mb/s in, 0 b/s out" });
    return asked;
}
