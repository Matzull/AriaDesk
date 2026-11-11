// Connection to aria2 and the live task lists.

import { Aria2 } from './rpc.js';
import { settings, rpcUrl } from './settings.svelte.js';
import { boot, isDesktop, invoke, notify, send } from './native.js';
import { speed, taskName } from './format.js';
import { t } from './i18n.js';

const KEYS = ['gid', 'status', 'totalLength', 'completedLength', 'uploadLength', 'downloadSpeed', 'uploadSpeed',
    'connections', 'numSeeders', 'seeder', 'dir', 'files', 'bittorrent', 'infoHash', 'errorCode', 'errorMessage',
    'followedBy', 'following', 'belongsTo', 'numPieces', 'pieceLength'];

const HISTORY = 90;

export const app = $state({
    status: 'idle', // idle | connecting | connected | error
    error: '',
    version: '',
    features: [],
    stat: { downloadSpeed: 0, uploadSpeed: 0, numActive: 0, numWaiting: 0, numStopped: 0 },
    active: [],
    waiting: [],
    stopped: [],
    history: [],
    toasts: [],
    incoming: null, // file or links sent to the new task dialog
    offerStart: false // ask whether to start aria2 in the background
});

let client = null;
let timer = null;
let lastStatus = null;
let refreshing = false;
let offered = false;

const LOCAL_HOSTS = ['localhost', '127.0.0.1', '::1', '[::1]'];

function failed(e) {
    app.status = 'error';
    app.error = e.network ? t('noResponse', { url: rpcUrl() }) : e.message;
    // offer to start aria2 once per session, only when nothing listens on this machine
    if (!e.network || !isDesktop || offered || !LOCAL_HOSTS.includes(settings.rpc.host.toLowerCase())) return;
    offered = true;
    if (boot.nativeConfig.autoStartAria2) {
        startAria2().then((error) => error && toast(error, 'error'));
    } else if (!boot.nativeConfig.execCommandOnStartup) {
        app.offerStart = true;
    }
}

/** Starts aria2c with the configured port and secret. Returns an error message, or null once connected. */
export async function startAria2() {
    const result = await invoke('start-aria2', Number(settings.rpc.port), settings.rpc.secret);
    if (result?.error) {
        return result.error === 'not-found' ? t('aria2NotFound') : t('aria2StartFailed', { message: result.error });
    }
    for (let i = 0; i < 20 && app.status !== 'connected'; i++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        await connect();
    }
    if (app.status !== 'connected') return app.error;
    toast(t('aria2Started'), 'ok');
    return null;
}

export const rpc = (method, ...params) => {
    if (!client) return Promise.reject(new Error(t('notConnected')));
    return client.call(method, ...params);
};

export function toast(message, kind = 'info') {
    const id = Math.random();
    app.toasts.push({ id, message, kind });
    setTimeout(() => (app.toasts = app.toasts.filter((x) => x.id !== id)), kind === 'error' ? 6000 : 3500);
}

export async function connect() {
    client?.close();
    clearTimeout(timer);
    lastStatus = null;
    app.status = 'connecting';
    app.active = [];
    app.waiting = [];
    app.stopped = [];
    app.history = [];
    const c = (client = new Aria2(rpcUrl(), settings.rpc.secret, () => refresh()));
    try {
        const v = await c.call('getVersion');
        if (c !== client) return;
        app.version = v.version;
        app.features = v.enabledFeatures;
        app.status = 'connected';
        app.error = '';
    } catch (e) {
        if (c !== client) return;
        failed(e);
    }
    loop();
}

function loop() {
    clearTimeout(timer);
    timer = setTimeout(async () => {
        await refresh();
        loop();
    }, app.status === 'connected' ? Math.max(500, settings.refreshInterval) : 5000);
}

export async function refresh() {
    if (!client || refreshing) return;
    refreshing = true;
    const c = client;
    try {
        const [active, waiting, stopped, stat] = await c.multicall([
            ['tellActive', KEYS], ['tellWaiting', 0, 1000, KEYS], ['tellStopped', 0, 1000, KEYS], ['getGlobalStat']
        ]);
        if (c !== client) return;
        if (app.status !== 'connected') {
            const v = await c.call('getVersion');
            app.version = v.version;
            app.features = v.enabledFeatures;
            app.status = 'connected';
            app.error = '';
        }
        app.active = Array.isArray(active) ? active : [];
        app.waiting = Array.isArray(waiting) ? waiting : [];
        app.stopped = Array.isArray(stopped) ? stopped.reverse() : [];
        if (!(stat instanceof Error)) {
            for (const k in stat) stat[k] = Number(stat[k]);
            app.stat = stat;
            app.history = [...app.history.slice(-HISTORY + 1), { d: stat.downloadSpeed, u: stat.uploadSpeed }];
        }
        detectFinished();
        if (isDesktop) {
            send('set-tray-tooltip', `AriaDesk\n↓ ${speed(app.stat.downloadSpeed)}  ↑ ${speed(app.stat.uploadSpeed)}`);
        }
    } catch (e) {
        if (c === client) failed(e);
    } finally {
        refreshing = false;
    }
}

function detectFinished() {
    const now = new Map();
    for (const task of [...app.active, ...app.waiting, ...app.stopped]) now.set(task.gid, task);
    if (lastStatus && settings.notifications) {
        for (const [gid, task] of now) {
            const before = lastStatus.get(gid);
            if (!before || before === task.status || before === 'complete' || before === 'error' || before === 'removed') continue;
            if (task.status === 'complete' && !task.followedBy) notify(t('downloadComplete'), taskName(task));
            if (task.status === 'error') notify(t('downloadError'), taskName(task));
        }
    }
    lastStatus = new Map([...now].map(([gid, task]) => [gid, task.status]));
}

const isStopped = (task) => ['complete', 'error', 'removed'].includes(task.status);

/** Runs one aria2 method for every task and reports failures. */
async function each(tasks, method) {
    if (!tasks.length) return;
    try {
        const results = await client.multicall(tasks.map((task) => [typeof method === 'function' ? method(task) : method, task.gid]));
        const failed = results.filter((r) => r instanceof Error);
        if (failed.length) toast(failed[0].message, 'error');
    } catch (e) {
        toast(e.message, 'error');
    }
    refresh();
}

export const pause = (tasks) => each(tasks.filter((x) => ['active', 'waiting'].includes(x.status)), 'forcePause');
export const resume = (tasks) => each(tasks.filter((x) => x.status === 'paused'), 'unpause');
export const remove = (tasks) => each(tasks, (task) => (isStopped(task) ? 'removeDownloadResult' : 'forceRemove'));

export async function command(method, ...params) {
    try {
        const result = await rpc(method, ...params);
        refresh();
        return result;
    } catch (e) {
        toast(e.message, 'error');
        throw e;
    }
}

/** Adds a stopped task again with the same URIs and options. */
export async function retry(tasks) {
    for (const task of tasks) {
        const uris = [...new Set((task.files || []).flatMap((f) => (f.uris || []).map((u) => u.uri)))];
        if (!uris.length || task.bittorrent) continue;
        try {
            const options = await rpc('getOption', task.gid);
            await rpc('addUri', uris, options);
            await rpc('removeDownloadResult', task.gid);
        } catch (e) {
            toast(e.message, 'error');
        }
    }
    refresh();
}

export const canRetry = (task) => isStopped(task) && !task.bittorrent && task.files?.some((f) => f.uris?.length);
