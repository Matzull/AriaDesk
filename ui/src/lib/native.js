// Platform layer: the Tauri backend on the desktop, browser APIs on the web.
// The desktop backend speaks a channel based protocol (see src-tauri/src/ipc.rs).

const tauri = window.__TAURI__;

export const boot = window.__ARIADESK_BOOT__ || {
    platform: 'web',
    os: 'web',
    sep: '/',
    initialRoute: null,
    globals: { version: __APP_VERSION__, buildCommit: __BUILD_COMMIT__, isDevMode: false },
    nativeConfig: {},
    runtime: [{ name: 'Browser', value: navigator.userAgent }]
};

export const isDesktop = !!tauri;

export function send(channel, ...args) {
    if (isDesktop) {
        tauri.core.invoke('ipc_send', { channel, args });
    }
}

export function invoke(channel, ...args) {
    return isDesktop ? tauri.core.invoke('ipc_invoke', { channel, args }) : Promise.resolve(null);
}

const listeners = {};
const registrations = [];

/** Resolves once every backend event listener registered so far is active. */
export const listening = () => Promise.all(registrations);

export function on(channel, callback) {
    if (!listeners[channel]) {
        listeners[channel] = [];
        if (isDesktop) {
            registrations.push(tauri.event.listen(channel, (event) => emitLocal(channel, ...(event.payload || []))));
        }
    }
    listeners[channel].push(callback);
    return () => listeners[channel].splice(listeners[channel].indexOf(callback) >>> 0, 1);
}

export function emitLocal(channel, ...args) {
    for (const callback of listeners[channel] || []) {
        callback(...args);
    }
}

/** An error meaning nothing answered at the address (as opposed to an error reply). */
export const networkError = () => Object.assign(new Error('network'), { network: true });

/** POSTs a JSON body. On the desktop the request is made by the backend, so CORS does not apply. */
export async function postJson(url, body, timeout) {
    if (isDesktop) {
        const result = await invoke('http', {
            url, method: 'POST', data: body, timeout, headers: { 'Content-Type': 'application/json' }
        });
        if (result.response.status <= 0) {
            throw networkError();
        }
        return result.response.data;
    }
    let response;
    try {
        response = await fetch(url, {
            method: 'POST', body: JSON.stringify(body), headers: { 'Content-Type': 'application/json' },
            signal: AbortSignal.timeout(timeout)
        });
    } catch {
        throw networkError();
    }
    return response.json();
}

const pageId = Math.random().toString(36).slice(2, 8);
let socketCount = 0;

/** A WebSocket with the browser's interface, backed by the Rust client on the desktop. */
export function openWebSocket(url, handlers) {
    if (!isDesktop) {
        const ws = new WebSocket(url);
        ws.onopen = handlers.open;
        ws.onclose = handlers.close;
        ws.onmessage = (e) => handlers.message(e.data);
        return { send: (data) => ws.send(data), close: () => { ws.onclose = null; ws.close(); } };
    }
    // the backend keeps one connection; the id keeps events and closes of older sockets apart
    const id = `${pageId}-${++socketCount}`;
    let offs = [];
    const dispose = () => {
        offs.forEach((off) => off());
        offs = [];
    };
    const closed = () => {
        dispose();
        handlers.close();
    };
    offs = [
        on('ws-open', (e) => e.id === id && handlers.open()),
        on('ws-close', (e) => e.id === id && closed()),
        // success is false when the backend could not send because the connection is gone
        on('ws-message', (e) => e.id === id && (e.success ? handlers.message(e.data) : closed()))
    ];
    listening().then(() => offs.length && send('ws-connect', url, id));
    return {
        send: (data) => send('ws-send', { id, data }),
        close: () => {
            dispose();
            send('ws-close', id);
        }
    };
}

/** Parses a .torrent (base64) with the shared Rust core. */
export function parseTorrent(base64) {
    const json = wasm && wasm.parseBittorrentInfo(base64, boot.sep || '/');
    return json ? JSON.parse(json) : null;
}

export function fileType(name) {
    const ext = (name.match(/\.[^.]+$/) || [''])[0].toLowerCase();
    return ext === '.torrent' ? 'torrent' : ext === '.meta4' || ext === '.metalink' ? 'metalink' : null;
}

export function openExternal(url) {
    if (isDesktop) {
        send('open-url', url);
    } else {
        window.open(url, '_blank', 'noopener');
    }
}

export function notify(title, body) {
    if (isDesktop) {
        send('notify', { title, body });
    } else if (window.Notification && Notification.permission === 'granted') {
        new Notification(title, { body });
    }
}

/** Desktop settings; kept in the boot data so the settings page can read them synchronously. */
export function setNativeConfig(key, value) {
    boot.nativeConfig[key] = value;
    send('set-config', key, value);
}
