// What the launcher's Nearby makes of the devices Bluetooth hears while
// it's open, in plain JavaScript so tests/bluetooth-pairing.qml can hold it
// with plain objects standing in for BlueZ's: which are listed and in what
// order, how strong each is heard, and what a pairing or connecting that
// failed was, from BlueZ's own error. Accessories.qml draws the rows and
// says the words, which are words.json's.
//
// A device is read as Quickshell.Bluetooth gives it, with the shell's patch
// (modules/quickshell/bluetooth-rssi-and-failures.patch): its `address`,
// `name`, `deviceName`, the name it gave itself, and `rssi`, in dBm, 0 when
// it hasn't been heard since discovery last stopped.
.pragma library
.import "Order.js" as Order

// A device's signal in dBm, from one too faint to hold to one beside the
// machine, for its bar: as Links.js draws a Low Energy link's.
const FAINTEST = -100;
const LOUDEST = -30;
// At or above each it's heard as its word says, strongest first; below the
// last, weak.
const STRONG = -60;
const FAIR = -75;
const STRENGTHS = [
    [STRONG, "strong"],
    [FAIR, "fair"]
];

const ERROR = "org.bluez.Error.";
// What each of BlueZ's errors for a pairing or connecting means, by its
// name after org.bluez.Error. (src/error.h); "" is no failure at all, a
// device paired or connected already.
const BY_ERROR = new Map([
    ["AuthenticationFailed", "passkey"],
    ["AuthenticationRejected", "refused"],
    ["AuthenticationCanceled", "stopped"],
    ["AuthenticationTimeout", "unreached"],
    ["ConnectionAttemptFailed", "unreached"],
    // biome-ignore lint/security/noSecrets: BlueZ's error name, not a secret.
    ["DoesNotExist", "unreached"],
    ["InProgress", "busy"],
    ["NotReady", "unready"],
    ["NotAvailable", "nothing"],
    ["AlreadyExists", ""],
    ["AlreadyConnected", ""]
]);
// A connection's failure is named in its message under
// org.bluez.Error.Failed, by src/error.h's br-connection and
// le-connection reasons, or by the kernel's own words.
const BY_REASON = [
    [/-refused$/, "refused"],
    [/-canceled$/, "stopped"],
    [/timeout|^host is down$/, "unreached"],
    [/profile-unavailable/, "nothing"]
];
const SPACED = /[-_]/g;
const CAPITAL = /([A-Z])/g;
const STOP = /\.$/;

// Devices are live states (Order.live), and each heard stands alike here,
// one Joel might pair, so the strongest heard comes first.
const strongest = Order.live({
    stands: () => 0,
    then: (a, b) => b.rssi - a.rssi,
    name: device => device.name
});

function isHeard(device) {
    return typeof device.rssi === "number" && device.rssi !== 0;
}

// The devices Nearby lists: every one heard since discovery began,
// strongest first, then A to Z, those alike in both in the order BlueZ
// gave them. The launcher keeps a list shown in the order it was shown
// as they move (Steady.js), so this orders only its first look and where
// a new one would go. A device that never said its name is left out, as
// nothing could find it and Joel couldn't tell it from the next.
function heard(devices) {
    return Order.sorted(devices.filter(device => isHeard(device) && isNamed(device)), strongest);
}

// Those heard that never said their names, strongest first, which Nearby
// counts after the rest rather than hiding, and lists by their addresses
// only when asked, as BlueZ names them.
function nameless(devices) {
    return Order.sorted(devices.filter(device => isHeard(device) && !isNamed(device)), strongest);
}

function isNamed(device) {
    return (device.deviceName ?? "") !== "";
}

// A device's signal as words say it, in dBm with a real minus.
function dbm(device) {
    return String(device.rssi).replace("-", "\u2212");
}

// How strong a device is heard: its share of the bar, 0 to 1, and the word
// for it, "strong", "fair" or "weak"; null while it isn't heard.
function strength(device) {
    if (!isHeard(device)) {
        return null;
    }
    const share = (device.rssi - FAINTEST) / (LOUDEST - FAINTEST);
    const word = STRENGTHS.find(each => device.rssi >= each[0])?.[1] ?? "weak";
    return { value: Math.max(0, Math.min(1, share)), word };
}

// What a failed pairing or connecting was, as the words that say it are
// keyed: "passkey" when the device wanted one typed, "refused" when it said
// no, "stopped" when it gave up, "unreached" when it couldn't be reached,
// "busy" when it was already being paired or connected, "unready" when
// the radio wasn't, "nothing" when it offers nothing this machine uses,
// and "other" for anything else, said in BlueZ's own words; "" for no
// failure at all.
function failure(name, message) {
    const error = (name ?? "").startsWith(ERROR) ? name.slice(ERROR.length) : "";
    if (BY_ERROR.has(error)) {
        return BY_ERROR.get(error);
    }
    const said = (message ?? "").toLowerCase();
    return BY_REASON.find(each => each[0].test(said))?.[1] ?? "other";
}

// BlueZ's own words for a failure "other" stands for, as the foot says
// them: its message, lowercase, its reason's dashes as spaces, with no
// full stop; or with no message, its error's own name, never "failed".
function plain(name, message) {
    const said = (message ?? "").trim();
    const error = (name ?? "").startsWith(ERROR) ? name.slice(ERROR.length) : name ?? "";
    const words = said === "" ? error.replace(CAPITAL, " $1").trim() : said;
    return words.replace(SPACED, " ").replace(STOP, "").toLowerCase();
}
