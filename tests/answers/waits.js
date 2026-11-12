// The cases of Lag for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how. What Lag says
// beside its name in Machine, in the words Joel reads (Waits.js).
.import "common.js" as Common
.import "../Waits.js" as Waits

function run(failures, fromRoot) {
    let asked = 0;
    const lag = Common.checker(failures, () => asked++);
    const words = JSON.parse(Common.read(fromRoot, "words.json")).things
        .find(thing => thing.id === "waits").said;
    const settings = { group: ",", point: "." };
    const said = window => Waits.headline(window, words, settings);
    const cargo = { of: "cpu", name: "cargo", share_ratio: 0.687, cores: 16 };
    lag("a wait on the processor, by its cause and the program most behind it",
        said({ trouble: ["the desktop waited for a CPU 3.0% of the time"], worst: "cpu",
            top: cargo }),
        "slowed by the processor \u00b7 cargo \u00b7 11 of 16 cores");
    lag("a program barely using it uses one core",
        said({ trouble: ["it waited"], worst: "cpu", top: Object.assign({}, cargo,
            { share_ratio: 0.01 }) }),
        "slowed by the processor \u00b7 cargo \u00b7 1 of 16 cores");
    lag("a wait on memory, by the program holding the most",
        said({ trouble: ["it waited", "it swapped"], worst: "memory",
            top: { of: "memory", name: "firefox", resident_b: 6_237_000_000 } }),
        "slowed by memory \u00b7 firefox \u00b7 6.24 GB");
    lag("a wait on the processor in its first window, its cause alone",
        said({ trouble: ["it waited"], worst: "cpu" }), "slowed by the processor");
    lag("a wait on the disk, its cause alone",
        said({ trouble: ["it waited"], worst: "disk" }), "slowed by the disk");
    lag("niri slow to answer", said({ trouble: ["it waited"], worst: "niri" }),
        "the desktop's slow to answer");
    lag("a cause it has no words for, in lag-watch's sentence",
        said({ trouble: ["it waited on something new"], worst: "fans" }),
        "it waited on something new");
    lag("nothing waiting", said({ trouble: [], worst: "" }), "nothing's slowing the desktop");
    return asked;
}
