// What the launcher's answers must make of what's typed, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-launcher-answers-what-is-asked` check copies Numbers.js,
// Addresses.js and the rest beside this file, with the character names
// character-names.py builds, and runs `qml -platform offscreen
// answers.qml`, with the shell's maths (Maths, libqalculate) on its import
// path, so sums and measures are worked out as the shell works them out.
// Each place's cases are in answers/<place>.js, and this runs each in turn,
// so two places' cases never share a file; a new place adds its file to
// both lists below. Each case names what's typed and what has to come back,
// or that nothing should.
import QtQuick
import Maths
import "answers/accessories.js" as AccessoriesCases
import "answers/addresses.js" as AddressesCases
import "answers/cameras.js" as CamerasCases
import "answers/characters.js" as CharactersCases
import "answers/clock.js" as ClockCases
import "answers/colours.js" as ColoursCases
import "answers/command-examples.js" as CommandExamplesCases
import "answers/core.js" as CoreCases
import "answers/heat.js" as HeatCases
import "answers/keep-awake.js" as KeepAwakeCases
import "answers/machine.js" as MachineCases
import "answers/measures.js" as MeasuresCases
import "answers/networks.js" as NetworksCases
import "answers/now-drawers.js" as NowDrawersCases
import "answers/power.js" as PowerCases
import "answers/shortcuts.js" as ShortcutsCases
import "answers/sound.js" as SoundCases
import "answers/subnets.js" as SubnetsCases
import "answers/sums.js" as SumsCases
import "answers/times.js" as TimesCases
import "answers/waits.js" as WaitsCases
import "answers/warp.js" as WarpCases

QtObject {
    Component.onCompleted: {
        const failures = [];
        // Read as this file finds them, where the check copies them. The maths
        // is given time enough, since its 50 ms patience is a key's budget, which
        // maths-speed.qml holds: on a loaded builder "5 parsecs in km" came late
        // and these cases, which judge what's answered, failed (2026-11-12).
        const patient = {
            calculate: (expression, waits) => Maths.calculate(expression, waits ?? 0)
        };
        const fromRoot = { maths: patient, url: file => Qt.resolvedUrl(file) };
        let asked = 0;
        const places = [AccessoriesCases, AddressesCases, CamerasCases, CharactersCases,
            ClockCases, ColoursCases, CommandExamplesCases, CoreCases, HeatCases,
            KeepAwakeCases, MachineCases, MeasuresCases, NetworksCases, NowDrawersCases,
            PowerCases, ShortcutsCases, SoundCases, SubnetsCases, SumsCases, TimesCases,
            WaitsCases, WarpCases];
        for (const place of places)
            asked += place.run(failures, fromRoot);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
