// How near a temperature is to the most it's meant to reach, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// Machine.qml draws its processor's and disk's temperature by it, as the
// machine daemon draws Heat's parts (modules/machine/src/parts.rs's level),
// so the two say them alike. Heat's own parts, rows and headline are the
// daemon's, served ready (Heat.qml).
.pragma library

// What a part is drawn against when its chip says no most: the disk's
// usual most, and a battery's, which wants to stay well below a
// processor's.
const USUAL = { disk: 85, battery: 60 };
const USUAL_MOST = 100;
// How near the most a temperature warns, where the chip says no lower
// point of its own.
const NEAR = 10;

function given(value) {
    return value !== null && value !== undefined;
}

// Where a temperature starts to warn: the chip's own high point where
// that's below its critical one, as a disk's is, and else ten degrees
// short of the critical, as a processor's high and critical are one; or
// nowhere, with neither said, since nothing says where trouble starts.
function warnsFrom(high, crit) {
    if (given(high) && !(given(crit) && high >= crit)) {
        return high;
    }
    if (given(crit)) {
        return crit - NEAR;
    }
    return null;
}

// How near a temperature is to the most it's meant to reach: its share
// of that most, or of its part's usual most where the chip says none,
// and whether it warns.
function level(value, high, crit, part) {
    const most = crit ?? high ?? USUAL[part] ?? USUAL_MOST;
    const from = warnsFrom(high, crit);
    return { value: Math.max(0, Math.min(1, value / most)), warn: from !== null && value >= from };
}
