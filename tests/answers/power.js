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
    const profileHolds = [{ profile: "performance", app: "Steam", reason: "a game" },
        { profile: "power-saver", app: "GNOME", reason: "" }];
    const choice = (daemon, known) => Power.choiceFacts(daemon, Object.assign({ holds: profileHolds,
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
    // What a lock leaves open, from the windows daemon's windows-open
    // record, counted and ordered there (ZINAWZ, #3718): the windows,
    // then whose in the daemon's order, four named and the windows of the
    // rest counted, a window of no program among them, so the figures
    // add up to the windows open (Y2YBTS, 2026-11-12).
    const openWords = { window: "1 window", windows: "{count} windows", more: "{count} more",
        silent: "not answering" };
    const open = (count, programs) => Power.openOf({ key: "windows-open", open: count,
        programs: programs.map(([program, many]) => ({ program, count: many })) });
    machine("what a lock leaves open", Power.openSaid(open(9, [["kitty", 3], ["Chrome", 2],
        ["mpv", 1], ["slack", 1], ["zed", 1]]), openWords),
    { windows: "9 windows", programs: "kitty 3 \u00b7 Chrome 2 \u00b7 mpv 1 \u00b7 slack 1"
        + " \u00b7 2 more" });
    machine("a window of no program is counted in the rest",
        Power.openSaid(open(3, [["kitty", 2]]), openWords),
        { windows: "3 windows", programs: "kitty 2 \u00b7 1 more" });
    machine("one window open", Power.openSaid(open(1, [["kitty", 1]]), openWords),
        { windows: "1 window", programs: "kitty 1" });
    machine("none open says nothing", Power.openSaid(open(0, []), openWords),
        { windows: "", programs: "" });
    // Not heard is not none: before the windows daemon answers, nothing
    // is known open, so no drawer says nothing closes (Y2YBTS, f7a6155a).
    machine("before the windows daemon answers, nothing is known open",
        Power.openOf(null), null);
    // Lock's drawer leads with what it does and what keeps running, then
    // its key and whose windows stay open; with none open, only that it
    // locks, and no key where it has none.
    const lockWords = Object.assign({ lead: "Locks the screen; {windows} keep running.",
        one: "Locks the screen; {windows} keeps running.", alone: "Locks the screen." },
    openWords);
    machine("Lock's drawer", Power.lockSaid(open(2, [["kitty", 2]]), "Super L", lockWords), { lead: "Locks the screen; 2 windows keep running.", facts: [
        { id: "session:Lock:key", value: "Super L" },
        { id: "session:Lock:open", value: "kitty 2" }] });
    machine("Lock's drawer with one window open says it keeps running",
        Power.lockSaid(open(1, [["kitty", 1]]), "Super L", lockWords).lead,
        "Locks the screen; 1 window keeps running.");
    machine("Lock's drawer with nothing open and no key",
        Power.lockSaid(open(0, []), "", lockWords),
        { lead: "Locks the screen.", facts: [] });
    machine("Lock's drawer before the windows daemon answers says only what it does",
        Power.lockSaid(null, "Super L", lockWords, false),
        { lead: "Locks the screen.", facts: [{ id: "session:Lock:key", value: "Super L" }] });
    machine("and once it isn't answering, says so of what stays open",
        Power.lockSaid(null, "Super L", lockWords, true).facts,
        [{ id: "session:Lock:key", value: "Super L" },
            { id: "session:Lock:open", value: "not answering" }]);
    // Ending the session's drawer says what it ends: how many windows in
    // its lead, and whose in its Ends; with none open, only what it does.
    const endWords = Object.assign({ lead: "Reboots the machine, closing {windows}.",
        alone: "Reboots the machine." }, openWords);
    machine("Reboot's drawer", Power.endingSaid(open(3, [["kitty", 2], ["Chrome", 1]]),
        "session:Reboot:ends", endWords),
    { lead: "Reboots the machine, closing 3 windows.", facts: [
        { id: "session:Reboot:ends", value: "kitty 2 \u00b7 Chrome 1" }] });
    machine("Reboot's drawer with nothing open",
        Power.endingSaid(open(0, []), "session:Reboot:ends", endWords),
        { lead: "Reboots the machine.", facts: [] });
    machine("Reboot's drawer before the windows daemon answers",
        Power.endingSaid(null, "session:Reboot:ends", endWords, false),
        { lead: "Reboots the machine.", facts: [] });
    machine("Reboot's drawer with the windows daemon not answering says so in Ends",
        Power.endingSaid(null, "session:Reboot:ends", endWords, true),
        { lead: "Reboots the machine.", facts: [
            { id: "session:Reboot:ends", value: "not answering" }] });
    // How bright a room is, from the light sensor's lux.
    machine("a dark room", Power.roomLevel(0), 0);
    machine("daylight", Power.roomLevel(10000), 1);
    machine("a room a hundredth of daylight", Math.round(Power.roomLevel(100) * 100) / 100,
        0.5);
    return asked;
}
