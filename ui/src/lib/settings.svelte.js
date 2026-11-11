// Frontend settings, persisted in localStorage.

const KEY = 'ariadesk.settings';

const defaults = {
    rpc: { protocol: 'http', host: 'localhost', port: 6800, path: '/jsonrpc', secret: '' },
    language: navigator.language?.toLowerCase().startsWith('es') ? 'es' : 'en',
    theme: 'system',
    refreshInterval: 1000,
    notifications: true,
    confirmRemove: true,
    sort: { key: 'added', desc: false },
    dirHistory: [],
    showChart: true
};

function load() {
    try {
        const saved = JSON.parse(localStorage.getItem(KEY)) || {};
        return { ...defaults, ...saved, rpc: { ...defaults.rpc, ...saved.rpc }, sort: { ...defaults.sort, ...saved.sort } };
    } catch {
        return structuredClone(defaults);
    }
}

export const settings = $state(load());

$effect.root(() => {
    $effect(() => {
        const json = JSON.stringify(settings);
        try {
            localStorage.setItem(KEY, json);
        } catch {
            // storage unavailable
        }
    });
});

export function rememberDir(dir) {
    if (!dir) return;
    settings.dirHistory = [dir, ...settings.dirHistory.filter((d) => d !== dir)].slice(0, 10);
}

export function rpcUrl(rpc = settings.rpc) {
    const path = rpc.path.startsWith('/') ? rpc.path : '/' + rpc.path;
    return `${rpc.protocol}://${rpc.host}:${rpc.port}${path}`;
}
