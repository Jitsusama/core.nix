// The launcher's copy of a mesh address's shape (launcher/Address.js), judged
// in Qt's own JavaScript engine by the cases every copy reads,
// modules/mycelium/site/tests/addresses.txt, read as its README's The
// Address Cases says. The `the-launcher-reads-the-address-cases` check
// copies Address.js, tests/lib/Expect.js and addresses.txt beside this file
// and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/Address.js" as Address

QtObject {
    Component.onCompleted: Expect.run(cases)

    // The escape a case may hold: 1 to 6 capital hex digits.
    readonly property var escaped: /\\u\{([0-9A-F]{1,6})\}/gu

    function cases() {
        refused();
        read();
        judged();
    }

    // A case's text with each escape made the character it names, throwing on
    // any other backslash or an escape naming no character.
    function caseText(written) {
        const line = JSON.stringify(written);
        if (written.replace(escaped, "").includes("\\"))
            throw new Error(`${line} should escape only as \\u{HEX}`);
        return written.replace(escaped, (_, hex) => {
            const code = Number.parseInt(hex, 16);
            if (code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff))
                throw new Error(`${line} should name a character`);
            return String.fromCodePoint(code);
        });
    }

    // The cases `written` holds, each [whether it is an address, its text],
    // throwing on a line written any way but one.
    function casesIn(written) {
        return written.split("\n")
            .filter(line => line !== "" && !line.startsWith("#"))
            .map(line => {
                if (line.includes("\r"))
                    throw new Error(`${JSON.stringify(line)} should hold no carriage return`);
                const [kind, ...rest] = line.split(" ");
                if (kind !== "is" && kind !== "not")
                    throw new Error(`${JSON.stringify(line)} should say is or not`);
                return [kind === "is", caseText(rest.join(" "))];
            });
    }

    function throws(written, reading) {
        try {
            reading(written);
        } catch (error) {
            return true;
        }
        return false;
    }

    // Each reader of the cases refuses what the README says it must.
    function refused() {
        for (const written of ["T\\", "T\\n", "T\\u{a}", "T\\u{+A}", "T\\u{0000041}",
            "T\\u{}", "T\\u{41", "T\\u{D800}", "T\\u{DFFF}", "T\\u{110000}", "T\\\\"])
            Expect.same(`${JSON.stringify(written)} is refused`,
                throws(written, caseText), true);
        for (const written of ["is T\r", "not T\r\n", "is T\n\r", "IS T", "maybe T"])
            Expect.same(`${JSON.stringify(written)} is refused`,
                throws(written, casesIn), true);
    }

    // What a case says is read the one way.
    function read() {
        Expect.same("each escape is read the one way", caseText("T\\u{5C}\\u{9}"), "T\\\t");
        Expect.same("six digits, the top and each side of the surrogates are read",
            caseText("\\u{000041}\\u{10FFFF}\\u{E000}\\u{D7FF}"),
            "A\u{10FFFF}\u{E000}\u{D7FF}");
        Expect.same("what an escape gives is never read again", caseText("T\\u{5C}u{41}"),
            "T\\u{41}");
        Expect.same("a note and an empty line are skipped, and not alone is the empty text",
            casesIn("# a note\nis T\n\nnot\nnot T "), [[true, "T"], [false, ""], [false, "T "]]);
    }

    // The launcher's copy says of each shared case what the case says.
    function judged() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("addresses.txt"), false);
        request.send();
        const shared = casesIn(request.responseText);
        Expect.same("the shared cases are read", shared.length > 20, true);
        for (const [is, text] of shared)
            Expect.same(`${JSON.stringify(text)} ${is ? "is" : "isn't"} an address`,
                Address.shaped(text), is);
        Expect.same("nothing but a string is one", Address.shaped(null), false);
    }
}
