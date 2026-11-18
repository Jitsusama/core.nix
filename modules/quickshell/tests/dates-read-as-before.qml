// Whether Date.parse still reads what it read, in the Qt the shell is built
// with, which refuses a string without a digit before trying every form
// Qt reads (modules/quickshell/qtdeclarative-dates-without-digits-refused.patch).
// The `dates-read-as-before` check runs this as the shell: ECMA's own
// format, Qt's text, ISO and RFC 2822 forms and a few of its others read
// the moment they say, strings no form reads give NaN, and five hundred
// empty strings take under 100 ms, since each took about 2 ms before. It
// exits 1, saying what was wrong, for any.
import Quickshell
import QtQuick

ShellRoot {
    Component.onCompleted: {
        const wrong = [];
        const expect = (what, got, wanted) => {
            if (!Object.is(got, wanted))
                wrong.push(`${what}: ${String(got)}, not ${String(wanted)}`);
        };
        // The check runs in UTC (TZ), so a form read as local time reads
        // the same moment as one read in UTC.
        const moment = Date.UTC(2026, 10, 13, 21, 59, 20);
        const day = Date.UTC(2026, 10, 13);

        expect("ECMA's format", Date.parse("2026-11-13T21:59:20Z"), moment);
        expect("ECMA's format with an offset", Date.parse("2026-11-13T16:59:20-05:00"), moment);
        expect("ECMA's date alone", Date.parse("2026-11-13"), day);
        expect("a fraction past milliseconds", Date.parse("2026-11-13T21:59:20.123456Z"),
            moment + 123);
        expect("Qt's text form", Date.parse("Fri Nov 13 21:59:20 2026"), moment);
        expect("RFC 2822", Date.parse("Fri, 13 Nov 2026 21:59:20 +0000"), moment);
        expect("a month by name", Date.parse("Nov 13 2026"), day);
        expect("a month by number", Date.parse("11/13/2026"), day);
        expect("a new Date said so", new Date("Nov 13, 2026 21:59:20").getTime(), moment);

        for (const said of ["", "undefined", "null", "soon", "now", " ", "T:Z"])
            expect(`${JSON.stringify(said)}`, Date.parse(said), NaN);
        expect("undefined itself", Date.parse(undefined), NaN);
        expect("a number in no form", Date.parse("5 minutes"), NaN);
        expect("a new Date of nothing", new Date("").getTime(), NaN);

        const began = Date.now();
        for (let at = 0; at < 500; at += 1)
            Date.parse("");
        const took = Date.now() - began;
        if (took >= 100)
            wrong.push(`500 empty strings took ${took} ms, at most 100`);

        for (const one of wrong)
            console.warn(`a date was read wrong: ${one}`);
        // Said once the shell has loaded, as before it nothing hears it.
        Qt.callLater(() => Qt.exit(wrong.length === 0 ? 0 : 1));
    }
}
