// The cases of the commands' worked examples for the launcher's answers,
// which ../answers.qml runs with every other place's; its opening comment
// says how. Each command's drawer shows an example or two, typed as the
// command starts, and what comes back (launcher/CommandRow.js). Where the
// answer is fixed, worked out in the shell's own JavaScript, the example
// is run through it here and has to say its headline word for word, since
// a wrong example is worse than none (VKCF3Z on #3781, 2026-11-12). Where
// it changes by the day or acts, the example describes what comes back
// instead, and every command's provider has to be one or the other, so a
// new command can't go unchecked.
.import "common.js" as Common
.import "../Addresses.js" as Addresses
.import "../Characters.js" as Characters
.import "../Numbers.js" as Numbers
.import "../Subnets.js" as Subnets
.import "../Units.js" as Units

// The providers whose answers change by the day or the place (money,
// time, weather, the theme's colours) or act (timers, sound, the screen's
// and the desk's light), whose examples describe what comes back.
const described = ["colours", "currency", "displays", "light", "sound", "time", "timers",
    "weather"];

// The answer each provider with a fixed one gives what's typed, as the
// shell works it out: the maths is the shell's, the names Unicode's. A
// hardware address's maker needs the IEEE's list, which this check hasn't,
// so its example is a locally set one, which names none.
function fixed(maths, names) {
    return {
        calculator: typed => Numbers.answer(typed, undefined, maths),
        characters: typed => Characters.answer(typed, names),
        networks: typed => Subnets.answer(typed),
        units: typed => Units.answer(typed, undefined, maths),
        vendors: typed => {
            const at = Addresses.mac(typed.trim().replace(/^mac\s+(?=\S)/i, ""));
            return at ? { headline: Addresses.maker(at, "") } : null;
        }
    };
}

function run(failures, fromRoot) {
    let asked = 0;
    const quoted = Common.quoted;
    const names = Characters.prepare(Common.read(fromRoot, "names.txt"));
    const answering = fixed(fromRoot.maths, names);
    const commands = JSON.parse(Common.read(fromRoot, "words.json")).commands;
    for (const command of commands) {
        const answer = answering[command.provider];
        if (!answer) {
            asked++;
            if (!described.includes(command.provider)) {
                failures.push(`${command.id}'s provider ${quoted(command.provider)} is neither`
                    + " worked out here nor said to change or act");
            }
            continue;
        }
        for (const example of command.examples ?? []) {
            asked++;
            const got = answer(example.typed);
            if (got?.headline !== example.answers) {
                failures.push(`${command.id}'s example "${example.typed}" says`
                    + ` ${quoted(example.answers)}, but it answers ${quoted(got?.headline)}`);
            }
        }
    }
    return asked;
}
