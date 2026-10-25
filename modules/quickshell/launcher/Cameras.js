// What the launcher's cameras make of the kernel's files, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// a loopback camera's format as its sysfs file says it, where a sensor sits
// as its firmware says, how long it's been powered, which of a camera's
// facts are the formats it offers, and which of its sensor's lights are
// which. Cameras.qml reads the files and draws the rows; nothing here opens
// a camera, since a camera opened is a camera in use.
.pragma library

// A loopback's format file: its four letters, its size, and its frames a
// second, whole or as a fraction ("NV12:1920x1080@300000/10000").
const LOOPBACK_FORMAT = /^(\w{1,4}):(\d+)x(\d+)@(\d+)(?:\/(\d+))?$/;
// A sensor on the I2C bus, by its firmware's id and instance ("i2c-OVTI08F4:00").
const SENSOR = /^i2c-([A-Z0-9]+):(\d+)$/;
// A format's name as desk-facts writes it (modules/desk-facts' format_name).
const FORMAT_NAME = /^[A-Z0-9][A-Z0-9.]*$/;
// The facts of a camera that aren't formats, by their ids.
const NOT_FORMATS = ["usb", "firmware", "streaming", "field of view"];
// The lights a sensor's power controller names, by the end of their names,
// and the part each is.
const LIGHTS = [["privacy_led", "privacy"], ["ir_flood_led", "flood"]];
// Formats as they're usually written, where their four letters differ.
const FORMATS = [["MJPG", "MJPEG"], ["H264", "H.264"], ["HEVC", "H.265"], ["H265", "H.265"]];
const SECOND = 1000;
const MINUTE = 60;
const HOUR = 3600;

function mode(width, height, rate) {
    return `${width} \u00d7 ${height} at ${rate}`;
}

// A format's four letters, as desk-facts says them.
function formatName(letters) {
    const known = FORMATS.find(each => each[0] === letters);
    return known ? known[1] : letters;
}

// What a loopback camera gives whoever reads it, from its sysfs format
// file, or "" when the file says nothing it can read.
function loopback(text) {
    const said = LOOPBACK_FORMAT.exec(String(text ?? "").trim());
    if (!said) {
        return "";
    }
    const per = Number(said[5] ?? 1);
    const rate = per > 0 ? Math.round(Number(said[4]) / per) : 0;
    return `${formatName(said[1])} ${mode(said[2], said[3], rate)}`;
}

// A sensor's firmware id and the prefix its lights are named by, from its
// folder's name on the bus, or null for anything else on it.
function sensor(folder) {
    const said = SENSOR.exec(folder);
    return said ? { id: said[1], lights: `${said[1]}_${said[2]}::` } : null;
}

// Which part a light is, by its name, for the sensor whose lights start so,
// or "" for a light that isn't the sensor's or isn't one it knows.
function light(name, prefix) {
    if (!name.startsWith(prefix)) {
        return "";
    }
    const known = LIGHTS.find(each => name.slice(prefix.length) === each[0]);
    return known ? known[1] : "";
}

// Where the firmware says a sensor sits, from its physical_location files
// as { panel, vertical, horizontal, lid }: "front, centre right". A centre
// is spelled as it's said here, and what the firmware doesn't know is left
// out.
function place(location) {
    const spelled = word => (word === "center" ? "centre" : word);
    const known = word => word && word !== "unknown";
    const across = [location.vertical, location.horizontal].filter(known).map(spelled);
    const middle = across.length === 2 && across[0] === across[1] ? [across[0]] : across;
    const said = [known(location.panel) ? location.panel : "", middle.join(" ")]
        .filter(part => part !== "").join(", ");
    return location.lid === "yes" && said !== "" ? `${said}, in the lid` : said;
}

// A time in milliseconds, said in its largest whole units.
function lasted(milliseconds) {
    const seconds = Math.floor(Math.max(0, milliseconds) / SECOND);
    if (seconds < MINUTE) {
        return `${seconds} s`;
    }
    if (seconds < HOUR) {
        return `${Math.floor(seconds / MINUTE)} min`;
    }
    const minutes = Math.floor(seconds % HOUR / MINUTE);
    return minutes === 0 ? `${Math.floor(seconds / HOUR)} h`
        : `${Math.floor(seconds / HOUR)} h ${minutes} min`;
}

// A camera's facts from desk-facts, split into the formats it offers, each
// its largest and fastest mode, and the rest, each kept in its order.
function formats(facts) {
    const all = facts ?? [];
    const isFormat = fact => FORMAT_NAME.test(fact.name)
        && !NOT_FORMATS.includes(fact.name.toLowerCase());
    return {
        modes: all.filter(isFormat),
        rest: all.filter(fact => !isFormat(fact))
    };
}
