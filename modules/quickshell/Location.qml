pragma Singleton
// Where the laptop is, the one place the shell learns it from: what
// whereabouts last found through GeoClue (whereabouts/), its town and the
// zone it keeps, the time zone the clock keeps now, and the clock's last
// move to follow the laptop (clock-follows/), all in the one record,
// `here:laptop`, whereabouts holds over its socket in the shell's daemon
// interface. The weather's here and Now's Location read it, and nothing
// else asks where the laptop is.
//
// It's watched from the start, since it changes only as the laptop moves
// or the clock does, and costs nothing between. Nothing is assumed: until
// whereabouts has answered, or while it isn't answering, here is unknown,
// and Now says so (Joel, 2026-10-28: "i never want it to assume
// anything").
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/location.sock`
        kind: "here"
    }

    readonly property var record: daemon.revision >= 0 ? daemon.held["here:laptop"] ?? null
        : null

    // Here, as the daemon says it, in the shape the weather and Now read:
    // { latitude, longitude, accuracy in metres, when in seconds, place,
    // zone, country, hand, and altitude, speed and heading where GeoClue
    // knows them }, place and zone null while they couldn't be asked; or
    // null where it isn't known.
    readonly property var here: hereOf(record)
    // Whether whereabouts has said once, or is known not to answer, so
    // "unknown" is said only once it's known to be.
    readonly property bool read: daemon.answered || daemon.state === "not answering"

    // The zone the clock keeps, "America/Toronto", or "" until whereabouts
    // has said.
    readonly property string clockZone: record?.clock_zone ?? ""

    // Whether the place keeps another zone than the clock does.
    readonly property bool zoneDiffers: here !== null && (here.zone ?? "") !== ""
        && clockZone !== "" && here.zone !== clockZone

    // The clock's last move to where the laptop is (clock-follows/), {
    // from, to, place, when }, kept until it's put back or moves again, or
    // null; said only while the clock still keeps the zone it moved to.
    readonly property var moved: movedOf(record?.moved)
    readonly property bool clockMoved: moved !== null && moved.to === clockZone

    // How each of whereabouts' sources stands, by name: the clock, GeoClue
    // and the naming of places, each with the daemon's word.
    readonly property var sources: daemon.sources

    // A zone by its city, as people say a zone's time: "Edmonton".
    function city(zone) {
        return String(zone).split("/").pop().split("_").join(" ");
    }

    // What's said of the clock's move while it keeps the zone it moved to,
    // or "": "clock moved to Edmonton time, as the laptop is in Kelpwater".
    readonly property string movedSaid: !clockMoved ? ""
        : `clock moved to ${city(moved.to)} time`
            + (moved.place ? `, as the laptop is in ${moved.place.split(",")[0]}` : "")

    // Here from the daemon's record, or null for nowhere or a record that
    // names no place on the Earth.
    function hereOf(said) {
        if (!Number.isFinite(said?.latitude) || !Number.isFinite(said.longitude))
            return null;
        const known = value => Number.isFinite(value) ? value : null;
        return {
            latitude: said.latitude,
            longitude: said.longitude,
            accuracy: known(said.accuracy_m),
            when: known(said.at_s),
            place: said.place ?? null,
            zone: said.zone ?? null,
            country: said.country ?? null,
            hand: said.hand === true,
            altitude: known(said.altitude_m),
            speed: known(said.speed_m_s),
            heading: known(said.heading_deg)
        };
    }

    function movedOf(said) {
        return typeof said?.from === "string" && typeof said.to === "string"
            ? { from: said.from, to: said.to, place: said.place ?? null, when: said.at_s ?? null }
            : null;
    }

    // Where Joel's own place is pinned (whereabouts' state folder), which
    // whereabouts says as here in place of GeoClue's while it's there.
    readonly property string pin: `${Quickshell.env("XDG_STATE_HOME")
        || `${Quickshell.env("HOME")}/.local/state`}/whereabouts/pin.json`

    // How setting it by hand went, for the foot: "" until asked, then
    // what's being found, what was, or why it wasn't.
    property string told: ""

    // Sets where the laptop is to the town Joel names, found by the web.
    function setByHand(words) {
        if (setter.running)
            return;
        told = `finding ${words}\u2026`;
        setter.asked = words;
        setter.command = ["whereabouts", "set", root.pin, words];
        setter.running = true;
    }

    // Takes Joel's place away, so GeoClue finds the laptop again, which
    // whereabouts hears at once.
    function findAgain() {
        told = "";
        findingAgain.running = true;
    }

    Process {
        id: findingAgain
        command: ["whereabouts", "again", root.pin]
    }

    Process {
        id: setter
        property string asked: ""
        // As whereabouts' sysexits say: no such place, or no network.
        readonly property int nowhere: 65
        readonly property int unasked: 69
        onExited: code => {
            const asked = setter.asked;
            root.told = code === 0 ? ""
                : code === setter.nowhere ? `no place is called ${asked}`
                : code === setter.unasked ? `couldn't ask where ${asked} is`
                : `${asked} couldn't be set`;
        }
    }

    // Puts the clock back in the zone it kept before it followed the
    // laptop, where it's held until the laptop is somewhere else.
    function putBack() {
        puttingBack.running = true;
    }

    Process {
        id: puttingBack
        command: ["systemctl", "--user", "start", "--no-block", "clock-put-back.service"]
    }
}
