// What the machine daemon says of the screen's backlight and the room's
// sensor (modules/machine's light.rs), in plain JavaScript so a check can
// run it in Qt's own engine, as the shell does, for the Brightness a step
// into a screen holds. A screen's drawer and its Room Light say the light
// as the windows daemon serves it (modules/windows, light.rs and
// light/room.rs).
.pragma library
.import "Order.js" as Order

const PERCENT = 100;

// The first of a kind of the machine daemon's records (light.rs), by its
// device's name as the kernel lists them, or null with none.
function first(records, kind, holds) {
    return Order.sorted(records.filter(record => record?.kind === kind && holds(record)),
        Order.byName(record => record.device))[0] ?? null;
}

// The screen's backlight, from the machine daemon's records: its steps
// now and most, its share, the steps the hardware reads back where they
// differ, as when firmware moved it, how its steps map to light, what
// drives it, its device and the screen it lights; or null with none that
// says the most it can be.
function lit(records) {
    const found = first(records, "backlight",
        record => record.max_brightness > 0 && Number.isFinite(record.brightness));
    if (found === null) {
        return null;
    }
    const reads = found.actual_brightness;
    return {
        now: found.brightness,
        most: found.max_brightness,
        share: Math.round(found.brightness / found.max_brightness * PERCENT),
        reads: Number.isFinite(reads) && reads !== found.brightness ? reads : null,
        stepping: found.scale ?? null,
        driven: found.type ?? null,
        device: found.device,
        connector: found.connector ?? null
    };
}

// The room's light sensor, from the machine daemon's records: the lux it
// hears, its raw count, the lux a count is, its offset and its device; or
// null with none. Its name is always als, the only one light.rs reads.
function sensor(records) {
    const found = first(records, "light", () => true);
    if (found === null) {
        return null;
    }
    return {
        lux: Number.isFinite(found.lux) ? found.lux : null,
        raw: Number.isFinite(found.raw) ? found.raw : null,
        perCount: found.scale,
        offset: found.offset ?? 0,
        device: found.device
    };
}
