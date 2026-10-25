// What the launcher's Keyboards answer makes of the kernel's files, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// which of the kernel's input devices are keyboards, how each is joined, the
// lock lights it has, where a USB board's own description sits in sysfs, how
// its lights' timeout reads, and which layers a board can be moved to.
// Keyboards.qml reads the files and says it, and Keyboard.qml offers the
// layers too.
.pragma library

// The kernel's buses, as /proc/bus/input/devices numbers them.
const USB = "0003";
const BLUETOOTH = "0005";
const PS2 = "0011";
// ZSA's vendor number, the ErgoDox's maker.
const ZSA = "3297";

// A blank line between two devices in the kernel's list, and the fields of
// each device's lines.
const BETWEEN = /\n\s*\n/;
const NAME = /^N: Name="(.*)"$/m;
const PHYS = /^P: Phys=(.*)$/m;
const SYSFS = /^S: Sysfs=(.*)$/m;
const UNIQ = /^U: Uniq=(.*)$/m;
const HANDLERS = /^H: Handlers=(.*)$/m;
const BUS = /^I: .*\bBus=(\w+)/m;
const VENDOR = /^I: .*\bVendor=(\w+)/m;
const PRODUCT = /^I: .*\bProduct=(\w+)/m;
const LEDS = /^B: LED=([0-9a-f]+)$/im;
const HEX = 16;

// What gives a device keys and lights, and keyd's own keyboard.
const KEYS = /\bkbd\b/;
const LIGHTS = /\bleds\b/;
const KEYD = /^keyd\b/;

// Where a keyboard sits: on USB, by a Bluetooth address, or the laptop's own
// PS/2 port.
const ON_USB = /^usb-/;
const ADDRESS = /^([0-9a-f]{2}:){5}[0-9a-f]{2}$/i;
const PORT = /^isa|serio|i8042/;

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
const LOCKS = [
    { id: "caps", bit: 1, light: "capslock" },
    { id: "num", bit: 0, light: "numlock" },
    { id: "scroll", bit: 2, light: "scrolllock" }
];

// What a pattern finds in a device's lines, or "".
function found(pattern, block) {
    return (pattern.exec(block) ?? [])[1] ?? "";
}

// Each device in the kernel's list, with what it says of itself.
function devices(text) {
    return String(text ?? "").split(BETWEEN).filter(block => block.trim() !== "")
        .map(block => ({
            name: found(NAME, block),
            phys: found(PHYS, block),
            sysfs: found(SYSFS, block),
            uniq: found(UNIQ, block),
            handlers: found(HANDLERS, block),
            bus: found(BUS, block).toLowerCase(),
            vendor: found(VENDOR, block).toLowerCase(),
            product: found(PRODUCT, block).toLowerCase(),
            leds: Number.parseInt(found(LEDS, block) || "0", HEX)
        }));
}

// The keyboards among them: a device the kernel gives both keys and lights,
// which leaves out the power button, the lid and the hotkeys. keyd's own
// keyboard only passes on the others' keys, so it's left out too.
function keyboards(text) {
    return devices(text).filter(device => KEYS.test(device.handlers)
        && LIGHTS.test(device.handlers) && !KEYD.test(device.name));
}

// How a keyboard is joined, from where the kernel says it sits.
function joined(device) {
    if (ON_USB.test(device.phys)) {
        return "by USB";
    }
    if (ADDRESS.test(device.phys)) {
        return "by Bluetooth";
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

// Which keyboard the laptop's own light is: the one on its keyboard port,
// or, on a laptop whose keyboard is joined another way (I2C, or USB
// inside), the first that's neither paired nor a board with its own
// lights; null with none.
function lightHolder(listed) {
    return listed.find(builtIn)
        ?? listed.find(device => joined(device) !== "by Bluetooth" && !zsa(device))
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
// "after {delay} ms, {rate} Hz".
function repeats(repeat, said) {
    return String(said).replace("{delay}", repeat.delay).replace("{rate}", repeat.rate);
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
