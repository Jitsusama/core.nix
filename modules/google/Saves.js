.pragma library
// What the shell says of each save the picker staged for Google Drive, as
// the google program reports them (src/drive.rs's Uploaded): which to tell,
// and the notice each is told as (VKCF3Z, 8b69fe8a and b91d1b31). Sent,
// it's a brief notice that answers what Joel just did; not sent, it stays
// missed until seen, saying why, with what mends it and Save a Copy, so
// nothing saved is lost.

// Each save is one asking, its staged path and when it was sent.
function key(upload) {
    return `${upload.path}@${upload.asked}`;
}

// Whether it's done one way or the other: up, or failed.
function settled(upload) {
    return (upload.id ?? null) !== null || (upload.failed ?? null) !== null;
}

// The saves to tell, oldest first: settled, asked since the shell started,
// so a restart says nothing twice, and not told yet.
function untold(uploads, since, told) {
    return (uploads ?? []).filter(upload => settled(upload) && upload.asked >= since
        && !told.includes(key(upload))).reverse();
}

// Where it went, as Joel knows the folder: My Drive for its top, else the
// folder's name from any listing that showed it, else nothing.
function where(upload, listings) {
    if (upload.folder === "root")
        return "My Drive";
    for (const listing of listings ?? []) {
        const found = (listing.files ?? []).find(file => file.id === upload.folder);
        if (found)
            return found.name;
    }
    return "";
}

const sent = "Saved to Google Drive";
const unsent = "Not Saved to Google Drive";
const copy = { key: "copy", name: "Save a Copy" };

// Why each failure stopped it, and the acts after the reason, the one that
// mends it first. A file that couldn't be read leaves nothing to keep.
const failures = {
    signedOut: { why: ": Google signed out", acts: [{ key: "sign-in", name: "Sign In" }, copy] },
    unreachable: {
        why: ": Drive couldn't be reached",
        acts: [{ key: "again", name: "Try Again" }, copy]
    },
    gone: { why: ": that folder is gone", acts: [copy] },
    replaceNotGranted: {
        why: ": Drive holds one you made elsewhere, which this can't replace",
        acts: [copy]
    },
    unread: { why: " couldn't be read, so nothing was saved", acts: [] }
};

// The notice a save is told as: { summary, body, acts, answers }, each act
// { key, name }, answers true when it only answers what Joel just did.
function notice(upload, listings) {
    if ((upload.failed ?? null) === null) {
        const place = where(upload, listings);
        return {
            summary: sent,
            body: place === "" ? upload.name : `${upload.name} \u00b7 ${place}`,
            acts: upload.link ? [{ key: "open", name: "Open in Drive" }] : [],
            answers: true
        };
    }
    const failure = failures[upload.failed] ?? failures.unreachable;
    return {
        summary: unsent,
        body: `${upload.name}${failure.why}`,
        acts: failure.acts,
        answers: false
    };
}

// Whether a notice in the history is the failure told of this save's file,
// to take back once the file is up or kept elsewhere.
function failedOf(notice, path) {
    const name = path.slice(path.lastIndexOf("/") + 1);
    return notice.app === "Google Drive" && notice.summary === unsent
        && Object.values(failures).some(failure => notice.body === `${name}${failure.why}`);
}
