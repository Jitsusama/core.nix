// The cases of power for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Power.js" as Power

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    const machine = Common.checker(failures, () => asked++);
    // The power profile's drawer: its holds one fact parted by dots,
    // why it's slowed in words, the firmware's and the processor's
    // settings as written, and the drivers last.
    const profileWords = { eyebrow: "on this machine \u00b7 power profile",
        held: "{app} for {reason}", lap: "on a lap", heat: "too hot" };
    const profileDrawn = Power.profileDrawer({
        holds: [{ app: "Steam", reason: "a game" }, { app: "Firefox", reason: "a video" }],
        slowed: "heat", firmware: "performance", offers: "cool quiet balanced performance\n",
        processor: "balance_performance", processorDriver: "intel_pstate",
        firmwareDrivers: ["SoC Power Slider", "dell-pc"]
    }, id => `[${id.slice("session:profile:".length)}]`, profileWords);
    machine("the power profile's drawer", [profileDrawn.eyebrow].concat(profileDrawn.facts
        .map(fact => `${fact.label} ${fact.value}`)), ["on this machine \u00b7 power profile",
        "[held] Steam for a game \u00b7 Firefox for a video", "[slowed] too hot",
        "[firmware] performance",
        "[offers] cool \u00b7 quiet \u00b7 balanced \u00b7 performance",
        "[processor] balance_performance", "[processor-driver] intel_pstate",
        "[firmware-driver] SoC Power Slider \u00b7 dell-pc"]);
    machine("an unheld profile on a machine whose firmware says nothing",
        Power.profileDrawer({ holds: [{ app: "Steam", reason: "" }], slowed: "",
            processorDriver: "amd-pstate-epp" }, id => id, profileWords).facts
            .map(fact => fact.value), ["Steam", "amd-pstate-epp"]);
    // A profile picked is read back from the daemon, and one it didn't
    // take says why: nothing heard, a program holding another, or the
    // daemon keeping its own.
    const profileTold = holds => ({ holds,
        named: { "power-saver": "Power Saver", performance: "Performance" },
        said: { silent: "power profile not set \u00b7 not answering",
            held: "{app} holds {profile}", kept: "{profile} kept" } });
    machine("a profile taken says nothing",
        Power.profileRefusal("balanced", "balanced", profileTold([])), "");
    machine("a daemon that didn't answer is said so",
        Power.profileRefusal("balanced", "", profileTold([])),
        "power profile not set \u00b7 not answering");
    machine("a program holding the profile kept is named",
        Power.profileRefusal("balanced", "performance",
            profileTold([{ profile: "performance", app: "Steam" }])),
        "Steam holds Performance");
    machine("else the daemon kept its own",
        Power.profileRefusal("balanced", "power-saver",
            profileTold([{ profile: "performance", app: "Steam" }])),
        "Power Saver kept");
    // How bright a room is, from the light sensor's lux.
    machine("a dark room", Power.roomLevel(0), 0);
    machine("daylight", Power.roomLevel(10000), 1);
    machine("a room a hundredth of daylight", Math.round(Power.roomLevel(100) * 100) / 100,
        0.5);
    return asked;
}
