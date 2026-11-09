// Which program a browser's app window belongs to, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `an-app-window-deeper-in-is-its-programs` check copies
// launcher/AppWindows.js beside this file and runs `qml -platform
// offscreen app-windows.qml` over desktop entries made up here, each
// saying the class of its own page's window as Google's programs do.
import QtQuick
import "launcher/AppWindows.js" as AppWindows

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    readonly property var entries: [
        { id: "google-chrome", startupClass: "" },
        { id: "google-mail", startupClass: "chrome-mail.google.com__-Default" },
        { id: "google-docs", startupClass: "chrome-docs.google.com__document_-Default" },
        { id: "google-sheets", startupClass: "chrome-docs.google.com__spreadsheets_-Default" },
        { id: "google-drive", startupClass: "chrome-drive.google.com__-Default" },
        { id: "google-meet", startupClass: "chrome-meet.google.com__-Default" },
        { id: "kitty", startupClass: "kitty" }
    ]

    function owner(appId) {
        return AppWindows.ownerOf(appId, entries)?.id ?? null;
    }

    property var failures: []
    property int asked: 0

    function expect(what, got, wanted) {
        asked += 1;
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    function run() {
        expect("a program's own page is its own",
            owner("chrome-docs.google.com__document_-Default"), "google-docs");
        expect("a new document, opened deeper in, is Docs'",
            owner("chrome-docs.google.com__document_create-Default"), "google-docs");
        expect("a document being edited is Docs' too",
            owner("chrome-docs.google.com__document_d_1aBc_edit-Default"), "google-docs");
        expect("a spreadsheet never goes to Docs, though they share a host",
            owner("chrome-docs.google.com__spreadsheets_create-Default"), "google-sheets");
        expect("a mail opened from the shell is Mail's",
            owner("chrome-mail.google.com__mail_u_0_-Default"), "google-mail");
        expect("a call, its code holding dashes, is Meet's",
            owner("chrome-meet.google.com__abc-defg-hij-Default"), "google-meet");
        expect("another profile's window is never this one's",
            owner("chrome-docs.google.com__document_create-Profile_1"), null);
        expect("a page no program keeps is nobody's",
            owner("chrome-calendar.google.com__calendar_r_eventedit-Default"), null);
        expect("a window that isn't a browser's app is nobody's here", owner("kitty"), null);
        expect("nor is one with no app id", owner(undefined), null);
        expect("nor is any, with no entries", AppWindows.ownerOf(
            "chrome-mail.google.com__-Default", undefined), null);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
