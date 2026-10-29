// How a battery that reports only its charge stands, worked out apart
// from the piece that says it so tests/crossing.qml can hold it in Qt's
// own engine. It's said once as it runs low and once more, urgent, as it
// comes to need Joel now (docs/design.md), never again for the same drop.
.pragma library

// How far past the low mark a charge has to rise before running low is
// news again, so a charge that wavers about the mark says nothing more.
const REARM = 5;

// How a charge stands against the marks alone: "critical" at or under
// `marks.critical` percent, "low" at or under `marks.low`, else "".
function against(charge, marks) {
    if (charge <= marks.critical) {
        return "critical";
    }
    return charge <= marks.low ? "low" : "";
}

// How it stands after a reading, given how it stood before. Once low it
// only goes deeper: Arrivals says each standing it hasn't just seen, so
// going back from critical to low would say it was low again. It's fine
// again only once it charges, or rises REARM points past the low mark.
function next(was, charge, charging, marks) {
    if (charging) {
        return "";
    }
    const now = against(charge, marks);
    if (was === "") {
        return now;
    }
    if (charge >= marks.low + REARM) {
        return "";
    }
    return was === "critical" || now === "critical" ? "critical" : "low";
}
