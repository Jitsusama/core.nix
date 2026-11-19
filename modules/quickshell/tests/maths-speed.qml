// How long the launcher's sums and measures take a key, through the shell's
// maths (Maths, libqalculate) as Calculator.qml and Units.qml ask it: each
// input typed a letter at a time, every start of it asked of both, as the
// launcher asks them on each key, many times over. The
// `the-launcher-sums-within-a-key` check copies Numbers.js and Units.js
// beside this file and runs it offscreen; it prints what each input cost a
// key and exits 1 when a typical key, the median input's, takes more than
// the budget, and 0 otherwise.
import QtQuick
import Maths
import "Numbers.js" as Numbers
import "Units.js" as Units

QtObject {
    // The most a typical key may take, in milliseconds: inside the
    // launcher's own budget for a search, with the frame left for drawing
    // (docs/design.md, Latency Budgets).
    readonly property real budget: 5
    // How many times each input is typed, so a millisecond clock can time it.
    readonly property int rounds: 40
    // What Joel types: sums, bases, bits, measures, and words that are
    // neither, which every key asks too.
    readonly property var typed: ["12*7", "(3+4)/2", "sqrt(2)", "2 * pi", "1,000,000 * 3",
        "255 in hex", "0x1f + 0b1010", "0b1011 << 3", "~5", "5 xor 3", "2^64", "20% of 50",
        "5 km in mi", "350 f in c", "25ms", "25' in m", "5 ft + 3 in in cm",
        "1 gib / (10 mbit/s) in min", "5 parsecs in km", "brightness 40", "battery",
        "time in tokyo", "10.0.0.0/22"]

    // Each key's answers for one input, typed a letter at a time.
    function typeOnce(text) {
        for (let end = 1; end <= text.length; end++) {
            const so = text.slice(0, end);
            Numbers.answer(so, {}, Maths);
            Units.answer(so, {}, Maths);
        }
    }

    Component.onCompleted: {
        const started = Date.now();
        Maths.prepare();
        Numbers.answer("1+1", {}, Maths);
        console.log(`definitions read and a first sum answered in ${Date.now() - started} ms`);
        const costs = [];
        for (const text of typed) {
            typeOnce(text);
            const from = Date.now();
            for (let round = 0; round < rounds; round++)
                typeOnce(text);
            const each = (Date.now() - from) / (rounds * text.length);
            costs.push(each);
            console.log(`${each.toFixed(3)} ms a key typing "${text}"`);
        }
        const sorted = costs.slice().sort((a, b) => a - b);
        const typical = sorted[Math.floor(sorted.length / 2)];
        const mean = costs.reduce((sum, cost) => sum + cost, 0) / costs.length;
        console.log(`typical ${typical.toFixed(3)} ms a key, mean ${mean.toFixed(3)} ms, `
            + `slowest ${sorted[sorted.length - 1].toFixed(3)} ms`);
        if (typical > budget) {
            console.error(`a typical key took ${typical.toFixed(3)} ms, past ${budget} ms`);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}
