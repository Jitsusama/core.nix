// Locking, keeping awake, holding notices, stopping a recording, the power
// profile and ending the session. These live here rather than on a key, so no chord ends the
// session by accident. While the machine is kept awake or notices are held,
// undoing it leads the home screen, since a mode should never hold unseen.
import qs
import Qt.labs.folderlistmodel
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick
import "Power.js" as Power

Provider {
    id: provider

    what: "session"

    // Locking and holding notices have keys too, so these rows stand for
    // those actions and draw their chords.
    readonly property string locks: "Lock the screen"
    readonly property string holds: "Hold notices, or let them show"
    draws: [locks, holds]

    function bound(does) {
        const action = Shortcuts.actions.find(candidate => candidate.does === does);
        return action ? action.chord : [];
    }

    // A thing's name, verb, words and glyph as the vocabulary says them
    // (Words.qml), in the state it's in.
    function said(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state),
            glyph: Words.glyph(key, state),
            // Its words' state, so the : line names the command for it.
            wordState: state
        }, fields));
    }

    readonly property var awake: awakeRow(KeepAwake.on, KeepAwake.lidShutToo)

    // Keep Awake as it stands, or as it will once systemd has started or
    // stopped what keeps it so: kept, a shut lid goes on too wherever the
    // machine's on the charger (KeepAwake.qml).
    function awakeRow(kept, lidToo) {
        return said("session:awake", kept ? "on" : "", {
            side: !kept ? "" : lidToo ? "awake, lid shut too" : "awake while open",
            hue: "machine",
            stays: true,
            run: () => provider.keepAwake(!kept)
        });
    }

    // Its row says so at once, until the services' files say so too
    // (launcher/Becoming.js).
    function keepAwake(kept) {
        becomes("session:awake", awakeRow(kept, KeepAwake.plugged), Words.name("session:awake"),
            kept ? "on" : "off");
        if (kept)
            KeepAwake.keep();
        else
            KeepAwake.release();
    }

    // The session's options, each named as VKCF3Z named it (2026-11-05).
    // :set dnd holds notices until it's ended, as an option stays on until
    // it's turned off, dnd=60 for an hour, dnd+=60 an hour more, and nodnd
    // ends it. :set keepawake keeps the machine awake and nokeepawake lets
    // it sleep; awake=open keeps it so only while the lid is open and
    // awake=shut with the lid shut too, where a shut lid would sleep it.
    // :set power=saver, balanced or performance chooses the power profile.
    // :set norecording stops what records, as Stop Recording does, and
    // recording records the whole screen, as Capture's first row does.
    setOptions: Object.assign({
        recording: {
            kind: "switch",
            // Off stops what records, as Stop Recording does.
            ends: true,
            now: () => Recorder.recording,
            set: to => {
                if (to && !Recorder.recording)
                    Recorder.screen();
                else if (!to && Recorder.recording)
                    Recorder.stop();
                return "";
            }
        },
        dnd: {
            kind: "switch",
            takes: "number",
            least: 1,
            now: () => provider.dndNow(),
            set: to => provider.dndTo(to)
        },
        keepawake: {
            kind: "switch",
            now: () => KeepAwake.on,
            set: to => {
                if (!to)
                    provider.keepAwake(false);
                else if (!KeepAwake.on)
                    provider.keepAwake(true);
                return "";
            }
        },
        power: {
            kind: "choice",
            values: profileChoices.map(choice => choice.option),
            now: () => profileChoices.find(choice => choice.profile === PowerProfiles.profile)
                ?.option ?? "",
            set: to => {
                PowerProfiles.profile = profileChoices.find(choice => choice.option === to)
                    .profile;
                return "";
            }
        }
    }, !KeepAwake.lidHolds ? {} : {
        awake: {
            kind: "choice",
            values: ["open", "shut"],
            now: () => KeepAwake.lidShutToo ? "shut" : "open",
            set: to => {
                if (to === "open")
                    KeepAwake.keepOpen();
                else
                    KeepAwake.keep();
                return "";
            }
        }
    })

    // Where Do Not Disturb stands, as :set dnd? says it: off, held until it's
    // ended, or the minutes left, rounded up so a hold never says 0.
    function dndNow() {
        if (!Quiet.on)
            return false;
        return Quiet.kept && Quiet.until > 0 ? Math.ceil((Quiet.until - Date.now()) / 60000)
            : true;
    }

    function dndTo(to) {
        if (to === false)
            Quiet.end();
        else if (to !== true)
            Quiet.keep(to);
        // dnd alone holds them until it's ended, though held for some
        // minutes or only while the screen's shared.
        else if (!Quiet.kept || Quiet.until > 0)
            Quiet.keep(0);
        return "";
    }

    // Do not disturb (Quiet.qml): Return holds notices for an hour, and the
    // right arrow offers holding them until it's ended.
    readonly property var quiet: said("session:quiet", Quiet.on ? "on" : "", {
        side: Quiet.on ? Quiet.lasts : "for an hour",
        chord: bound(holds),
        hue: "voice",
        stays: true,
        run: () => Quiet.on ? Quiet.end() : Quiet.keep(60),
        verbs: Quiet.on ? null : () => [60, 0].map(minutes => provider.said(
            `session:quiet:${minutes}`, "", {
                verb: "hold",
                glyph: minutes > 0 ? Theme.glyph.timed : Theme.glyph.quiet,
                hue: "action",
                stays: true,
                run: () => Quiet.keep(minutes)
            }))
    })

    // The power profile is a setting: one part saying which is chosen, its
    // choices a step in, each named there as the part's value reads
    // ("Balanced") and found from anywhere by its own name ("Balanced
    // power") or its words ("eco"), since the launcher lists a part of
    // another (`of`) inside it and finds it outside only by a word past
    // the whole's name. The charger's coming and going picks one again.
    // What's learned knows each by the name it had first (was), so a
    // rename keeps it.
    readonly property var profileChoices: [
        { profile: PowerProfile.PowerSaver, was: "Power Saver", option: "saver" },
        { profile: PowerProfile.Balanced, was: "Balanced Power", option: "balanced" },
        { profile: PowerProfile.Performance, was: "Performance", option: "performance" }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile)
    readonly property var profiles: profileChoices.map(choice => {
        const key = `session:power:${choice.was}`;
        return said(key, "", {
            side: PowerProfiles.profile === choice.profile ? "chosen" : "",
            hue: "machine",
            of: Words.of(key).of,
            part: Words.of(key).part,
            stays: true,
            run: () => PowerProfiles.profile = choice.profile
        });
    })
    readonly property var profileChosen: {
        const choice = profileChoices.find(each => each.profile === PowerProfiles.profile);
        return choice ? `session:power:${choice.was}` : "";
    }
    readonly property var profile: thingRow({
        thing: true,
        key: "session:profile",
        name: Words.name("session:profile"),
        side: profileChosen !== "" ? Words.of(profileChosen).part : "",
        glyph: profileChosen !== "" ? Words.glyph(profileChosen) : Words.glyph("session:profile"),
        // Who holds it and what the firmware and processor were told, under
        // the row (Power.js's profileDrawer).
        drawer: () => provider.profileDrawer()
    }, () => [])

    // What the power profile's drawer read of sysfs: read just after the
    // drawer asks, since reading moves the reader its binding would follow,
    // and again each second while the launcher is open, as
    // power-profiles-daemon writes the firmware's and the processor's files
    // after it says the profile changed.
    property var profileRead: null
    property bool profileAsked: false

    function profileDrawer() {
        if (profileRead === null) {
            Qt.callLater(() => provider.readProfile());
            return null;
        }
        const slowed = PowerProfiles.degradationReason;
        return Object.assign({ title: Words.name("session:profile") }, Power.profileDrawer(
            Object.assign({
                holds: Array.from(PowerProfiles.holds ?? []).map(hold => ({
                    app: hold.applicationId,
                    reason: hold.reason
                })),
                slowed: slowed === PerformanceDegradationReason.LapDetected ? "lap"
                    : slowed === PerformanceDegradationReason.HighTemperature ? "heat" : ""
            }, profileRead),
            id => Words.name(id),
            Words.of("session:profile:drawer").said));
    }

    function readProfile() {
        const drivers = [];
        for (let i = 0; i < platformProfiles.count; i++)
            drivers.push(sysfs(`${platformProfiles.get(i, "filePath")}/name`));
        const policy = "/sys/devices/system/cpu/cpufreq/policy0";
        const read = {
            firmware: sysfs("/sys/firmware/acpi/platform_profile"),
            offers: sysfs("/sys/firmware/acpi/platform_profile_choices"),
            processor: sysfs(`${policy}/energy_performance_preference`),
            processorDriver: sysfs(`${policy}/scaling_driver`),
            firmwareDrivers: drivers.filter(name => name !== "")
        };
        profileAsked = true;
        if (JSON.stringify(read) !== JSON.stringify(profileRead))
            profileRead = read;
    }

    Timer {
        id: profileReading

        interval: 1000
        repeat: true
        running: provider.profileAsked && (provider.launcher?.open ?? false)
        // Forgotten once running has settled, since running reads profileAsked
        // and clearing it while running changes is a binding loop.
        onRunningChanged: {
            if (!running)
                Qt.callLater(() => {
                    if (profileReading.running)
                        return;
                    provider.profileRead = null;
                    provider.profileAsked = false;
                });
        }
        onTriggered: provider.readProfile()
    }

    // What a file in /sys says, or "" when it says nothing.
    function sysfs(file) {
        probe.path = file;
        return read(probe).trim();
    }

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }

    // The firmware's platform profile drivers, listed by Qt as they change.
    FolderListModel {
        id: platformProfiles
        folder: "file:///sys/class/platform-profile"
        showFiles: false
        showDotAndDotDot: false
    }

    // Starting a recording is Capture's (Capture.qml). While anything
    // records, stopping it leads the home screen, so it never runs unseen.
    readonly property var recording: said("session:recording", "", {
        side: `recording ${Recorder.what}`,
        hue: "machine",
        stays: true,
        run: () => {
            // Stopped, it goes at once, as it will once its process ends.
            becomes("session:recording", null,
                ((Words.field.becoming ?? {}).objects ?? {}).recording ?? "", "stop");
            Recorder.stop();
        }
    })

    // What ending the session closes, said before it's done.
    readonly property string closing: {
        // Niri's windows are kept by id.
        const open = Object.keys(Niri.windows).length;
        return open === 0 ? "" : `, Closing ${open === 1 ? "the Window" : `${open} Windows`}`;
    }

    // What's shown of them says how they stand now: Return on a setting
    // keeps the launcher, and its row says what it changed to (Launcher.qml's
    // refreshed). Ending the session takes Joel out, so those close it.
    onThingsChanged: changed()

    // Each is learned by the name it had first (was), so renaming one keeps
    // what Joel taught it.
    things: [awake, quiet].concat(Recorder.recording ? [recording] : [], [profile], profiles, [
        { was: "Lock", does: locks, run: () => provider.launcher.lock.lock() },
        { was: "Suspend", run: () => Niri.spawn(["systemctl", "suspend"]) },
        { was: "Log Out", run: () => Niri.act({ Quit: { skip_confirmation: true } }) },
        { was: "Reboot", run: () => Niri.spawn(["systemctl", "reboot"]) },
        { was: "Power Off", run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => {
        const key = `session:${action.was}`;
        const sure = Words.of(key).sure ?? "";
        const asked = Words.of(key).asked ?? "";
        return said(key, "", {
            chord: action.does ? bound(action.does) : [],
            hue: "machine",
            run: action.run,
            // Those that end the session ask once, as a second Return, since
            // everyday words now find them.
            confirm: sure !== "",
            // Asked, it says what it does to what, then what it closes, as
            // "Reboot the Machine, Closing 12 Windows" (Confirm.question).
            asked: asked !== "" ? () => asked + provider.closing : undefined,
            verbs: sure !== "" ? () => [provider.thing({
                key: `${key}:sure`,
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
                verb: Words.verb(key),
                name: sure + provider.closing,
                glyph: Words.glyph(key),
                hue: "alert",
                run: action.run
            })] : null
        });
    }))
}
