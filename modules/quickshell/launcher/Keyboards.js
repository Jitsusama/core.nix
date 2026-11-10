// What the launcher's Keyboards answer makes of the keyboards connected, in
// plain JavaScript so a check can run it in Qt's own engine, as the shell
// does: how each is joined, the lock lights it has, where a USB board's own
// description sits in sysfs, how its lights' timeout reads, and which layers
// a board can be moved to. Which of the kernel's input devices are keyboards
// is the machine daemon's to say (modules/machine/src/keyboards.rs), as
// ConnectedKeyboards.qml hears it. Keyboards.qml says it, and Keyboard.qml
// offers the layers too.
.pragma library

// The kernel's buses, as /proc/bus/input/devices numbers them.
const USB = "0003";
const BLUETOOTH = "0005";
const PS2 = "0011";
// ZSA's vendor number, the ErgoDox's maker.
const ZSA = "3297";

// Where a keyboard sits: on USB, by a Bluetooth address, or the laptop's own
// PS/2 port.
const ON_USB = /^usb-/;
const ADDRESS = /^([0-9a-f]{2}:){5}[0-9a-f]{2}$/i;
const PORT = /^isa|serio|i8042/;

// A rate as desk-facts says it, "1000 a second", and the place before
// each three of its digits.
const A_SECOND = /^(\d+) a second$/;
const THOUSANDS = /\B(?=(\d{3})+(?!\d))/g;

// A USB device's own folder in sysfs, as "1-2" or "3-1.4" names it, above
// its interfaces ("1-2:1.0").
const USB_DEVICE = /^\d+-[\d.]+$/;
// A USB version as sysfs says it, " 2.00".
const VERSION = /^\s*(\d+)\.(\d)/;
const MEGA = 1000;

// A Dell's lights' timeout ("1m", "30s"), and the words of its triggers.
const TIMEOUT = /^(\d+)([smhd]?)$/;
const UNITS = { "": "second", s: "second", m: "minute", h: "hour", d: "day" };
const SPACES = /\s+/;
const WAKERS = { keyboard: "a key", touchpad: "the touchpad" };

// The lock lights an input device can have, by their bit in its LED mask
// and the name the kernel gives each one's light, in the order a part
// lists them: caps lock first, as it's what's asked.
// How it's connected when that says no more than the bus a drawer's
// eyebrow names.
const BUSES = ["USB", "Bluetooth"];
const LOCKS = [
    { id: "caps", bit: 1, light: "capslock" },
    { id: "num", bit: 0, light: "numlock" },
    { id: "scroll", bit: 2, light: "scrolllock" }
];

// How a keyboard is joined, from where the kernel says it sits, as its
// drawer's eyebrow says it (VKCF3Z, 2026-11-08).
function joined(device) {
    if (ON_USB.test(device.phys)) {
        return "on USB";
    }
    if (ADDRESS.test(device.phys)) {
        return "on Bluetooth";
    }
    if (PORT.test(device.phys)) {
        return "built in";
    }
    return "";
}

// Whether it's the board this machine is built around, the one whose
// lights are its keyboard light's.
function builtIn(device) {
    return joined(device) === "built in";
}

// Whether it's ZSA's, as the ErgoDox is, whose layer its watcher says.
function zsa(device) {
    return device.bus === USB && device.vendor === ZSA;
}

// Whether the board's layout is the one to show: only while a ZSA board is
// among the keyboards connected now and its layout is known, since a
// layout for keys that aren't there only misleads.
function boarded(listed, board) {
    return listed.some(zsa) && board.length > 0;
}

// Which layers Super+K shows of those connected now: the board's while
// it's boarded, and otherwise the laptop's own, as its keyboard always is.
function layout(listed, board, laptop) {
    return boarded(listed, board) ? board : laptop;
}

// Which keyboard the laptop's own light is: the one on its keyboard port,
// or, on a laptop whose keyboard is joined another way (I2C, or USB
// inside), the first that's neither paired nor a board with its own
// lights; null with none.
function lightHolder(listed) {
    return listed.find(builtIn)
        ?? listed.find(device => joined(device) !== "on Bluetooth" && !zsa(device))
        ?? null;
}

// The lock lights it has, each with its light's folder under the kernel's
// lights, as "input0::capslock".
function locks(device) {
    const input = device.sysfs.split("/").pop();
    return LOCKS.filter(lock => (device.leds & (1 << lock.bit)) !== 0)
        .map(lock => ({ id: lock.id, light: `${input}::${lock.light}` }));
}

// A USB board's own folder in sysfs, which names its maker and model, or ""
// for one that isn't on USB.
function usbFolder(device) {
    if (device.bus !== USB) {
        return "";
    }
    const steps = device.sysfs.split("/");
    let last = -1;
    steps.forEach((step, at) => {
        if (USB_DEVICE.test(step)) {
            last = at;
        }
    });
    return last < 0 ? "" : `/sys${steps.slice(0, last + 1).join("/")}`;
}

// A USB link's speed as sysfs says it in Mb/s ("12", "480", "5000").
function speed(said) {
    const mbps = Number(said);
    if (!(mbps > 0)) {
        return "";
    }
    return mbps >= MEGA ? `${mbps / MEGA} Gb/s` : `${mbps} Mb/s`;
}

// How it's connected, more closely than how it's joined: its USB version
// and speed, the Bluetooth address it answers at, or the laptop's own
// PS/2 port. `usb` is what its USB folder says, { version, speed }.
function connection(device, usb) {
    if (device.bus === USB) {
        const version = VERSION.exec(usb?.version ?? "");
        const fast = speed(usb?.speed);
        return [version ? `USB ${version[1]}.${version[2]}` : "USB", fast]
            .filter(part => part !== "").join(" at ");
    }
    if (device.bus === BLUETOOTH) {
        return ADDRESS.test(device.uniq) ? `Bluetooth, ${device.uniq.toUpperCase()}` : "Bluetooth";
    }
    if (device.bus === PS2) {
        return "PS/2";
    }
    return "";
}

// A light's level: off, or how many of its steps.
function lit(now, most) {
    return now === 0 ? "off" : `${now} of ${most}`;
}

// How long the lights stay on once nothing's touched, as a Dell's
// stop_timeout says it ("1m", "30s"), in words; none at all is never.
function timeout(said) {
    const text = String(said ?? "").trim();
    const parts = TIMEOUT.exec(text);
    if (!parts) {
        return text;
    }
    const count = Number(parts[1]);
    if (count === 0) {
        return "never";
    }
    const unit = UNITS[parts[2]];
    return count === 1 ? `1 ${unit}` : `${count} ${unit}s`;
}

// What lights them again, from a Dell's start_triggers ("+keyboard
// +touchpad"), as a note: "lit again by a key or the touchpad".
function wakers(said) {
    const on = String(said ?? "").split(SPACES).filter(word => word.startsWith("+"))
        .map(word => WAKERS[word.slice(1)] ?? word.slice(1));
    return on.length === 0 ? "" : `lit again by ${on.join(" or ")}`;
}

// How a held key repeats, as repeat.nix sets it, in the vocabulary's words
// (keyboards:repeat's "said"), which give it in Hz as every rate is:
// "after {delay} ms \u00b7 {rate} Hz".
function repeats(repeat, said) {
    return String(said).replace("{delay}", repeat.delay).replace("{rate}", repeat.rate);
}

// Where a keyboard is, for its drawer's eyebrow: by the bus it's on, or
// built in, as the vocabulary's keyboards' words key them.
function where(device) {
    if (ON_USB.test(device.phys)) {
        return "usb";
    }
    if (ADDRESS.test(device.phys)) {
        return "bluetooth";
    }
    return PORT.test(device.phys) ? "builtIn" : "";
}

// A rate desk-facts says as "1000 a second", as every rate is said: in
// Hz, its thousands apart.
function hertz(said) {
    const parts = A_SECOND.exec(String(said ?? "").trim());
    return parts === null ? String(said ?? "") : `${parts[1].replace(THOUSANDS, ",")} Hz`;
}

// A keyboard's drawer, from what Keyboards.qml read of it (`known`):
// where it is and what it is above its name; what moves first, the layer
// and the locks, then what it has counted; then how it's set, its polling
// rate, repeat and lights; then what describes it, how it's joined, its
// maker, model, keys and firmware; its identifiers last, its USB id and
// serial (VKCF3Z, 2026-11-07); and the keys it presses most closing it,
// under their heading. Each fact is named by its id (`named`), and one it
// doesn't say is left out, never said as unknown. The locks are one fact
// naming those on, since three saying "off" say nothing, and how it's
// connected only when it says more than the eyebrow's bus (VKCF3Z,
// 2026-11-08).
function drawer(known, named, said) {
    const desk = new Map((known.desk ?? []).map(one => [one.name, one.value]));
    const told = new Set(["polling rate", "keys", "firmware"]);
    const fact = (id, value) => ({ label: named(id), value: String(value ?? "") });
    const layer = known.layer === null || known.layer === undefined ? []
        : [fact("layer", known.layer.locked ? `${known.layer.name} \u00b7 ${said.locked}`
            : known.layer.name)];
    const lights = known.locks ?? [];
    const on = lights.filter(lock => lock.on).map(lock => said[lock.id] ?? lock.id);
    const locked = lights.length === 0 ? [] : [fact("locks", on.length === 0 ? said.none
        : on.join(" \u00b7 "))];
    const link = BUSES.includes(known.connection) ? "" : known.connection;
    const off = [known.timeout ?? "", known.wakers ?? ""].filter(one => one !== "")
        .join(" \u00b7 ");
    const facts = layer.concat(
        locked,
        known.counted ?? [],
        [
            fact("polling-rate", hertz(desk.get("polling rate"))),
            fact("repeat", known.repeat),
            fact("lights", known.lights),
            fact("timeout", off),
            fact("connection", link),
            fact("maker", known.maker),
            fact("model", known.model),
            fact("keys", desk.get("keys")),
            fact("firmware", desk.get("firmware"))
        ],
        // Anything else desk-facts comes to say of it, by its own name.
        (known.desk ?? []).filter(one => !told.has(one.name))
            .map(one => fact(one.name, one.value)),
        // Its identifiers in the Name face (DrawerPage.faceOf).
        [fact("usb-id", known.usbId), fact("serial", known.serial)]
            .map(one => Object.assign(one, { identifies: true }))
    ).filter(one => one.heading !== undefined || one.keys !== undefined
        || String(one.value ?? "").trim() !== "");
    return {
        eyebrow: said[where(known.device)] ?? "",
        title: known.name,
        facts: facts.concat(known.most ?? [])
    };
}

// The layers a board can be moved to, by their names in its order, as the
// launcher offers them: the one it's locked on let go, then each above the
// first locked on, all but the one it's locked on already. Each is the
// vocabulary's id, the key learning knows it by, the layer it names and the
// number the board is told. The first layer is where a board let go falls
// back to, so it's never locked on.
function layerChoices(names, now, locked) {
    const release = locked
        ? [{ id: "keyboard:release", key: "keyboard:release", layer: now, index: 0 }]
        : [];
    const locking = names.map((name, index) => ({
        id: "keyboard:lock",
        key: `keyboard:lock:${name}`,
        layer: name,
        index,
    })).filter((choice) => choice.index > 0 && !(locked && choice.layer === now));
    return release.concat(locking);
}
