// How the launcher's journal names what counted most (launcher/Most.js), in
// Qt's own engine, the one the shell runs it in. The
// `the-launcher-says-what-counted-most` check copies launcher/Most.js into
// launcher/ and tests/lib/Expect.js beside this file, and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Most.js" as Most

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    function cases() {
        Expect.same("the most first", Most.said({ windows: 4, emoji: 1914, runs: 100 }, 8),
            "emoji 1914, runs 100, windows 4");
        Expect.same("a tie by name", Most.said({ wifi: 2, sound: 2, agents: 2 }, 8),
            "agents 2, sound 2, wifi 2");
        Expect.same("no more than asked", Most.said({ a: 3, b: 2, c: 1 }, 2), "a 3, b 2");
        Expect.same("none that counted nothing", Most.said({ a: 0, b: 1, c: -1 }, 8), "b 1");
        Expect.same("nothing said for none", Most.said({ a: 0 }, 8), "");
        Expect.same("nothing said for nothing", Most.said({}, 8), "");
    }
}
