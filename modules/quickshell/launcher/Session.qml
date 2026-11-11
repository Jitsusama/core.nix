// Locking, keeping awake, holding notices, stopping a recording, the power
// profile and ending the session. These live here rather than on a key, so no chord ends the
// session by accident. While the machine is kept awake or notices are held,
// undoing it leads the home screen, since a mode should never hold unseen.
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick
import "Power.js" as Power
import "Interfaces.js" as Interfaces
import "Awake.js" as Awake
import "Captured.js" as Captured

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
    // stopped what keeps it so: kept, a shut lid goes on too, on battery as
    // on the charger (KeepAwake.qml). A switch always says how
    // it stands, so off says "off" (VKCF3Z, 2026-11-08).
    function awakeRow(kept, lidToo) {
        const howKept = lidToo ? "awake, lid shut too" : "awake while open";
        return said("session:awake", kept ? "on" : "", Object.assign({
            side: kept ? howKept : "off",
            hue: "machine",
            stays: true,
            run: () => provider.keepAwake(!kept)
        }, awakeFields()));
    }

    // A lock's holder as its program names itself, where the desktop has
    // an entry by the name logind was given ("1Password" for 1password),
    // and otherwise as it was given: an entry by id rather than a
    // heuristic, since a guess would rename NetworkManager for a settings
    // program.
    function programNamed(who) {
        return (DesktopEntries.byId(who) ?? DesktopEntries.byId(who.toLowerCase()))?.name || who;
    }

    // What Keep Awake's drawer says once logind has answered: who else
    // holds its locks, when an idle machine sleeps, who puts sleep off and
    // when it last slept (Awake.js). A switch has no drawer
    // (Launcher.qml's drawerOf), but Keep Awake holds what logind says
    // keeps the machine up or lets it sleep, so while it has figures to
    // show it is a thing, its act still keeping awake or letting it sleep
    // (VKCF3Z, 2026-11-08).
    function awakeFields() {
        const logind = awakeWatch.revision >= 0 ? awakeWatch.held["awake:main"] ?? null : null;
        if (!logind)
            return {};
        const words = Words.of("session:awake").said ?? {};
        const now = Date.now();
        const facts = Awake.facts(logind, words, {
            plugged: !UPower.onBattery,
            when: at => Captured.taken(at, now, provider.days),
            lasted: Interfaces.lasted,
            named: programNamed
        });
        return {
            thing: facts.length > 0,
            drawer: {
                eyebrow: words.eyebrow ?? "",
                eyebrowNames: true,
                title: Words.name("session:awake"),
                facts: facts.map(fact => ({ label: Words.name(fact.id), value: fact.value })),
                body: ""
            }
        };
    }

    // The words for a day a file's row says (FileFacts.js), so Last Slept
    // says a day as a thing taken does.
    readonly property var days: ({
        yesterday: (Words.of("file-chooser:changed").said ?? {}).yesterday,
        date: (at, format) => Qt.formatDate(at, format)
    })

    // Its row says so at once, until the services' files say so too
    // (launcher/Becoming.js).
    function keepAwake(kept) {
        becomes("session:awake", awakeRow(kept, kept), Words.name("session:awake"),
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
            now: () => profileChoices.find(choice => provider.profileOn(choice))?.option ?? "",
            set: to => provider.pickProfile(profileChoices.find(choice => choice.option === to))
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
    // Each is named too as power-profiles-daemon names it (`daemon`).
    readonly property var profileChoices: [
        { profile: PowerProfile.PowerSaver, was: "Power Saver", option: "saver",
            daemon: "power-saver" },
        { profile: PowerProfile.Balanced, was: "Balanced Power", option: "balanced",
            daemon: "balanced" },
        { profile: PowerProfile.Performance, was: "Performance", option: "performance",
            daemon: "performance" }
    ].filter(choice => choice.profile !== PowerProfile.Performance
        || PowerProfiles.hasPerformanceProfile)
    // Return on one never closes the launcher, from a step into it too
    // (`alwaysStays`), since it changes something where Joel is and its
    // row says so (2026-11-08: a pick from a step closed the launcher and
    // was lost on a laptop, unsaid). They're made again once the daemon's
    // said what it's on, so a pick held chosen is let go of, and one it
    // didn't take says why on its own row until the next pick.
    readonly property var profiles: profileChoices.map(choice => {
        void provider.profileHeard;
        const key = `session:power:${choice.was}`;
        const refusal = provider.profileRefused;
        return said(key, "", {
            side: refusal.daemon === choice.daemon ? refusal.why
                : provider.profileOn(choice) ? "chosen" : "",
            hue: "machine",
            of: Words.of(key).of,
            part: Words.of(key).part,
            stays: true,
            alwaysStays: true,
            run: () => provider.pickProfile(choice)
        });
    })

    // The profile last picked, while the daemon's asked what it's on, and
    // what it said, by its own name; "" until it's said.
    property var profilePicked: null
    property string profileHeard: ""
    // The last pick the daemon didn't take, by its name, and why.
    property var profileRefused: ({ daemon: "", why: "" })
    // The pick a read of the daemon was started for, so its answer is
    // judged against that pick, never one made while it ran.
    property var profileReadFor: null

    // Whether the daemon's on a profile: as it said when it last refused a
    // pick, until the next, since Quickshell's copy says what was asked;
    // else as Quickshell's copy says.
    function profileOn(choice) {
        return provider.profileRefused.daemon !== "" ? provider.profileHeard === choice.daemon
            : PowerProfiles.profile === choice.profile;
    }

    // Picks a profile, by Return or :set power=: said in the journal, its
    // row chosen at once and held so until the daemon says it's on it, then
    // asked a moment later; one it isn't on says why, in the foot and on
    // its row, and
    // one never said back says so once the hold's run out. Gives what :set
    // says of it.
    function pickProfile(choice) {
        const key = `session:power:${choice.was}`;
        const name = Words.of(key).part;
        console.info(`launcher: set power profile ${choice.option}`);
        provider.profilePicked = choice;
        provider.profileRefused = { daemon: "", why: "" };
        provider.profileHeard = "";
        becomes(key, { side: "chosen" }, name, "set", {
            // One a newer pick overtook is let go of, unsaid.
            done: () => provider.profilePicked !== choice
                || provider.profileHeard === choice.daemon
        });
        PowerProfiles.profile = choice.profile;
        profileAsking.restart();
        return Power.filled(Words.of("session:profile").said.set, { profile: name });
    }

    // How long after a pick the daemon's asked what it's on: a second, so
    // a refusal is said within two, well inside the launcher's wait for
    // any act to be agreed, so a slow answer's why still reaches the foot.
    readonly property int profileAsksAfter: 1000

    // The daemon's answer for the pick it was asked about, by its own
    // name, "" while it doesn't answer; one a newer pick has overtaken is
    // let be, its own read already timed.
    function profileToldBack(heard) {
        const picked = provider.profileReadFor;
        provider.profileReadFor = null;
        if (picked === null || picked !== provider.profilePicked)
            return;
        const why = Power.profileRefusal(picked.daemon, heard, {
            holds: Array.from(PowerProfiles.holds ?? []).map(hold => ({
                profile: profileChoices.find(choice => choice.profile === hold.profile)?.daemon
                    ?? "",
                app: hold.applicationId
            })),
            named: profileChoices.reduce((names, choice) => Object.assign(names, {
                [choice.daemon]: Words.of(`session:power:${choice.was}`).part
            }), {}),
            said: Words.of("session:profile").said
        });
        provider.profileHeard = heard;
        if (why === "") {
            console.info(`launcher: power profile ${picked.option} taken`);
        } else {
            console.warn(`launcher: power profile ${picked.option} not taken: ${why}`);
            provider.profileRefused = { daemon: picked.daemon, why };
            // Said while the pick's still held, so the foot says it too.
            refused(`session:power:${picked.was}`, why);
        }
        provider.profilePicked = null;
    }

    Timer {
        id: profileAsking
        interval: provider.profileAsksAfter
        onTriggered: {
            provider.profileReadFor = provider.profilePicked;
            provider.profileToldBack(profileDaemon.revision >= 0
                ? profileDaemon.held["profile:main"]?.active ?? "" : "");
        }
    }

    // What power-profiles-daemon says it's on, as the machine daemon hears
    // it on the system bus each time it says it changed
    // (modules/machine/src/kept.rs), rather than Quickshell's copy, which
    // says what was asked, so what's judged is the daemon's own. Nothing
    // is started per pick, and the watch wakes nothing between changes.
    Daemon {
        id: profileDaemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "profile"
    }
    readonly property var profileChosen: {
        const choice = profileChoices.find(each => provider.profileOn(each));
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

    // How many firmware platform profile drivers are looked for, the kernel
    // naming each platform-profile-<n> from 0.
    readonly property int profileDrivers: 8

    // What the power profile's drawer reads, only while it's asked: sysfs
    // says nothing of a change, so nothing is watched while it isn't.
    function readProfile() {
        const drivers = [];
        for (let n = 0; n < profileDrivers; n++) {
            const name = sysfs(`/sys/class/platform-profile/platform-profile-${n}/name`);
            if (name === "")
                break;
            drivers.push(name);
        }
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

    // Whether a row of this key is listed, so the machine daemon is asked
    // for its figures only while it can be seen: in the open launcher, or
    // among the rows a terminal's show walked or listed (Launcher.qml's
    // terminalSees), so a cold `launcher show` of Power has them.
    function rowShown(key) {
        if (launcher === null)
            return false;
        const keyed = row => row?.key === key;
        const terminal = launcher.terminalSees ?? { walked: [], listed: [] };
        return launcher.open && launcher.matches.some(keyed) || terminal.walked.some(keyed)
            || terminal.listed.some(keyed);
    }
    // Sleep's row, in Power or found by name: how sleep stands.
    readonly property bool sleepShown: rowShown("session:Suspend")
    // Keep Awake's row, in Power, at home or found by name: logind's locks.
    readonly property bool awakeShown: rowShown("session:awake")
    // Whether a watch hasn't yet answered whole, so a terminal's show
    // asks again (Provider.stillReading).
    stillReading: [sleepWatch, awakeWatch].some(watch => watch.live
        && !watch.answeredThisWatch && watch.state !== "not answering")
    readonly property var sleeping: sleepWatch.revision >= 0
        ? Power.sleepOf(sleepWatch.held["sleep:main"] ?? null) : null

    // What Sleep's row says of how sleep stands: the mode it sleeps in,
    // the step it last failed at, and its drawer (Power.js's sleepSaid).
    // An act has no drawer (Launcher.qml's drawerOf), but Sleep holds
    // what Linux counts of every sleep, so while it has figures to show it
    // is a thing whose act is Return (VKCF3Z, 2026-11-08). Its side says
    // how sleep stands, so it carries no ":suspend"; the foot says that.
    function sleepFields() {
        if (sleeping === null)
            return {};
        const told = Power.sleepSaid(sleeping, Words.of("session:Suspend").said ?? {},
            Interfaces.lasted, Words.of("sleep:error").said ?? {});
        return {
            thing: told.facts.length > 0,
            carries: false,
            side: told.side,
            note: told.note,
            // Its eyebrow says where its figures come from, then what
            // kind of thing it is, as every drawer's does (VKCF3Z,
            // 2026-11-08).
            drawer: {
                eyebrow: (Words.of("session:Suspend").said ?? {}).eyebrow ?? "",
                eyebrowNames: true,
                title: Words.name("session:Suspend"),
                facts: told.facts.map(fact => ({
                    label: Words.name(fact.id),
                    value: fact.value,
                    identifies: fact.identifies === true
                })),
                body: ""
            }
        };
    }

    Daemon {
        id: sleepWatch

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "sleep"
        live: provider.sleepShown
        keptThroughGap: false
        answersWhole: true
    }

    // What logind says keeps the machine up or lets it sleep, the machine
    // daemon's awake:main.
    Daemon {
        id: awakeWatch

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "awake"
        live: provider.awakeShown
        keptThroughGap: false
        answersWhole: true
    }

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
        { was: "Suspend", run: () => Niri.spawn(["systemctl", "suspend"]), fields: sleepFields() },
        { was: "Log Out", run: () => Niri.act({ Quit: { skip_confirmation: true } }) },
        { was: "Reboot", run: () => Niri.spawn(["systemctl", "reboot"]) },
        { was: "Power Off", run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => {
        const key = `session:${action.was}`;
        const sure = Words.of(key).sure ?? "";
        const asked = Words.of(key).asked ?? "";
        return said(key, "", Object.assign({
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
        }, action.fields ?? {}));
    }))
}
