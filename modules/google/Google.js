// What the shell reads of the Google account the google program keeps
// (src/state.rs), from the one line it leaves in google.json. Google.qml
// holds what's read, and the checks run it in Qt's own engine
// (tests/mail.qml).
.pragma library

// What the program said, or null when the line isn't its state, so a
// half-written line or a stray one changes nothing. Mail, the unread
// count and the documents are optional, as a program from before them
// doesn't say them.
function heard(line) {
    let said;
    try {
        said = JSON.parse(line);
    } catch (_error) {
        return null;
    }
    if (!said || typeof said.account !== "string" || !Array.isArray(said.meetings)) {
        return null;
    }
    return {
        account: said.account,
        meetings: said.meetings,
        mail: Array.isArray(said.mail) ? said.mail : [],
        unread: typeof said.unread === "number" ? said.unread : null,
        documents: Array.isArray(said.documents) ? said.documents : [],
        heard: said.heard ?? null
    };
}
