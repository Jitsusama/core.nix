// What making a text ready to match costs in memory, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-makes-texts-ready-lightly` check copies Fuzzy.js beside this
// file and runs `qml -platform offscreen fuzzy-light.qml` in a fresh process,
// with QML_XHR_ALLOW_FILE_READ set so it can read its own /proc. The shell
// makes a text ready for every row it lists and keeps up to 40,000 of them,
// so each one's cost is the shell's: a text grown a letter at a time, or a
// string made for each letter, took this process's memory up by 14 KB a
// text, where making them at once takes 7.
import QtQuick
import QtQuick.Window
import "Fuzzy.js" as Fuzzy

Window {
    visible: false

    // How many texts are kept: the most one lot of Fuzzy.kept holds.
    readonly property int texts: 20000
    // The most a kept text may raise the process's own memory, in bytes.
    readonly property int most: 10240

    // The process's own anonymous memory, in bytes.
    function anonymous() {
        const request = new XMLHttpRequest();
        request.open("GET", "file:///proc/self/smaps_rollup", false);
        request.send();
        const kilobytes = /Pss_Anon:\s+(\d+)/.exec(request.responseText);
        if (!kilobytes)
            throw new Error("/proc/self/smaps_rollup says no Pss_Anon");
        return Number(kilobytes[1]) * 1024;
    }

    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const before = anonymous();
        // Shaped like what the launcher lists: names, addresses, titles,
        // folders and hyphenated words.
        for (let i = 0; i < texts; i++)
            Fuzzy.kept(i % 2 === 0 ? `Ada Lovelace-Byron ${i} ada.${i}@example.com`
                : `A quest about the launcher, number ${i}, in ~/src/core.nix`);
        gc();
        const each = Math.round((anonymous() - before) / texts);
        console.info(`${texts} texts kept, ${each} bytes each`);
        if (each > most) {
            console.warn(`FAIL a kept text took ${each} bytes, more than ${most}`);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}
