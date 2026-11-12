// The cases of power for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Power.js" as Power

function run(failures) {
    let asked = 0;
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
    // A profile's own drawer: the one on instead while it isn't, its own
    // holds alone, Full Performance's being held back on its drawer
    // alone, and what it set only while it's on.
    const holds = [{ profile: "performance", app: "Steam", reason: "a game" },
        { profile: "power-saver", app: "GNOME", reason: "" }];
    const choice = (daemon, known) => Power.choiceFacts(daemon, Object.assign({ holds,
        slowed: "heat", current: "Balanced", firmware: "balanced",
        processor: "balance_performance" }, known), profileWords)
        .map(fact => `${fact.id.slice("session:profile:".length)} ${fact.value}`);
    machine("a profile not on says which is, who holds it and why it's held back",
        choice("performance", { on: false }),
        ["now Balanced", "held Steam for a game", "slowed too hot"]);
    machine("a profile on says what it set, and a hold given no reason by its program",
        choice("power-saver", { on: true }),
        ["held GNOME", "firmware balanced", "processor balance_performance"]);
    machine("a profile on before the firmware's been read says nothing it hasn't",
        choice("balanced", { on: true, firmware: undefined, processor: undefined }), []);
    // What a lock leaves open: the windows, then whose, the most first,
    // then by name, four named and the rest counted.
    const openWords = { window: "1 window", windows: "{count} windows", more: "{count} more" };
    const windows = apps => apps.reduce((all, app, id) => Object.assign(all,
        { [id]: { app_id: app } }), {});
    const named = id => ({ kitty: "kitty", "google-chrome": "Chrome" })[id] ?? id;
    machine("what a lock leaves open", Power.openSaid(Power.byProgram(windows(["kitty",
        "google-chrome", "kitty", "slack", "zed", "mpv", "google-chrome", "kitty", ""]),
    named), openWords), { windows: "9 windows", programs: "kitty 3 \u00b7 Chrome 2 \u00b7 mpv 1"
        + " \u00b7 slack 1 \u00b7 1 more" });
    machine("one window open", Power.openSaid(Power.byProgram(windows(["kitty"]), named),
        openWords), { windows: "1 window", programs: "kitty 1" });
    machine("none open says nothing", Power.openSaid(Power.byProgram({}, named), openWords),
        { windows: "", programs: "" });
    // Lock's drawer leads with what it does and what keeps running, then
    // its key and whose windows stay open; with none open, only that it
    // locks, and no key where it has none.
    const lockWords = Object.assign({ lead: "Locks the screen; {windows} keep running.",
        alone: "Locks the screen." }, openWords);
    machine("Lock's drawer", Power.lockSaid(Power.byProgram(windows(["kitty", "kitty"]), named),
        "Super L", lockWords), { lead: "Locks the screen; 2 windows keep running.", facts: [
        { id: "session:Lock:key", value: "Super L" },
        { id: "session:Lock:open", value: "kitty 2" }] });
    machine("Lock's drawer with nothing open and no key",
        Power.lockSaid(Power.byProgram({}, named), "", lockWords),
        { lead: "Locks the screen.", facts: [] });
    // How bright a room is, from the light sensor's lux.
    machine("a dark room", Power.roomLevel(0), 0);
    machine("daylight", Power.roomLevel(10000), 1);
    machine("a room a hundredth of daylight", Math.round(Power.roomLevel(100) * 100) / 100,
        0.5);
    return asked;
}
