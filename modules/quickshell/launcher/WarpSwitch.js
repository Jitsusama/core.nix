// What the launcher's WARP switch says: whether it takes WARP down or
// brings it up, who holds a switch the organisation holds, when WARP
// always on comes back once taken down, and what `warp connect` or `warp
// disconnect` said it did (modules/warp/src/switch.rs). Warp.qml runs
// the switch and draws its row; plain reading, held in Qt's own engine
// (tests/answers.qml).
.pragma library
.import "Warp.js" as Warp

// Whether WARP is up or on its way, so its switch takes it down.
function up(record) {
    return ["Connected", "Connecting"].includes(record?.status?.state ?? "");
}

// Who holds a switch the organisation holds, by its name.
function holds(organisation, said) {
    return Warp.worded(said, "holds", { organisation: Warp.text(organisation)
        ? organisation : Warp.worded(said, "theOrganisation") });
}

// When WARP always on comes back once taken down, after `minutes`.
function comesBack(minutes, settings, said) {
    if (!Number.isFinite(minutes)) {
        return "";
    }
    return minutes > 0
        ? Warp.worded(said, "comesBack", { minutes: Warp.worded(said, "minutes",
            { figure: Warp.count(minutes, settings) }) })
        : Warp.worded(said, "comesBackNow");
}

// What WARP's switch says beside it: who holds it, where the organisation
// does, or, while it's up and always on, when it would come back once
// taken down; nothing otherwise.
function switchNote(record, isUp, said) {
    const set = record?.settings ?? {};
    if (set.switch_locked === true) {
        return holds(set.organisation, said);
    }
    return isUp && set.always_on === true ? comesBack(set.auto_connect_after_min, {}, said) : "";
}

// Why a switch wasn't done, in warp's words where it gave them.
function why(printed, said) {
    return Warp.text(printed?.said) ? printed.said : Warp.worded(said, "couldntSwitch");
}

// What a switch came to, from what `warp connect` or `warp disconnect`
// printed: { refused, note }, why it wasn't done where it wasn't, or
// when WARP comes back by itself where it will.
function switched(printed, said) {
    if (printed?.act === undefined) {
        return { refused: why(printed, said), note: "" };
    }
    if (printed.refused === "held") {
        return { refused: holds(printed.organisation, said), note: "" };
    }
    if (Warp.text(printed.refused)) {
        return { refused: why(printed, said), note: "" };
    }
    return { refused: "", note: comesBack(printed.comes_back_after_min, {}, said) };
}
