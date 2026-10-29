// The extension's one worker: it connects to chrome-open-tabs, our
// native-messaging host (../host), as Chrome starts or installs it, which
// keeps the worker alive while the port is open, and answers each ask the
// host passes on from the launcher: which tabs are open in Chrome's own
// windows, and going to or closing one.
const HOST = "jitsusama.chrome_open_tabs";

let port = null;

function connect() {
    if (port !== null) {
        return;
    }
    port = chrome.runtime.connectNative(HOST);
    port.onMessage.addListener((asked) => {
        answer(asked).catch(() => said(asked, { done: false }));
    });
    port.onDisconnect.addListener(() => {
        port = null;
    });
}

function said(asked, what) {
    port?.postMessage({ id: asked.id, ...what });
}

async function answer(asked) {
    if (asked.ask === "list") {
        // A site's own app window, such as Calendar's, isn't a tab.
        const tabs = await chrome.tabs.query({ windowType: "normal" });
        said(asked, {
            tabs: tabs.map((tab) => ({
                id: tab.id,
                window: tab.windowId,
                active: tab.active,
                lastAccessed: tab.lastAccessed ?? 0,
                address: tab.url ?? "",
                title: tab.title ?? "",
            })),
        });
    } else if (asked.ask === "go") {
        const tab = await chrome.tabs.update(asked.tab, { active: true });
        await chrome.windows.update(tab.windowId, { focused: true });
        said(asked, { done: true });
    } else if (asked.ask === "close") {
        await chrome.tabs.remove(asked.tab);
        said(asked, { done: true });
    }
}

chrome.runtime.onStartup.addListener(connect);
chrome.runtime.onInstalled.addListener(connect);
connect();
