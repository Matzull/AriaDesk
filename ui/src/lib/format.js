const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];

export function bytes(value) {
    let n = Number(value) || 0;
    let i = 0;
    while (n >= 1024 && i < UNITS.length - 1) {
        n /= 1024;
        i++;
    }
    return `${i === 0 ? n : n.toFixed(n >= 100 ? 0 : n >= 10 ? 1 : 2)} ${UNITS[i]}`;
}

export const speed = (value) => bytes(value) + '/s';

export function duration(seconds) {
    if (!isFinite(seconds) || seconds < 0) return '∞';
    seconds = Math.round(seconds);
    const d = Math.floor(seconds / 86400);
    const h = Math.floor((seconds % 86400) / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (d) return `${d}d ${h}h`;
    if (h) return `${h}h ${m}m`;
    if (m) return `${m}m ${s}s`;
    return `${s}s`;
}

export function percent(done, total) {
    total = Number(total);
    return total > 0 ? Math.min(100, (Number(done) / total) * 100) : 0;
}

export function basename(path) {
    return String(path || '').split(/[\\/]/).filter(Boolean).pop() || '';
}

/** Display name of an aria2 task. */
export function taskName(task) {
    const bt = task.bittorrent;
    if (bt?.info?.name) return bt.info.name;
    const file = task.files?.[0];
    if (file?.path) {
        return task.files.length > 1 && task.dir ? basename(task.dir) || basename(file.path) : basename(file.path);
    }
    const uri = file?.uris?.[0]?.uri;
    if (uri) return basename(decodeURIComponent(uri.split('?')[0])) || uri;
    if (task.infoHash) return 'magnet:' + task.infoHash.slice(0, 12) + '…';
    return task.gid;
}

/** Path of a file relative to the task directory. */
export function relativePath(task, path) {
    const dir = String(task.dir || '').replace(/[\\/]+$/, '');
    return dir && path.startsWith(dir) ? path.slice(dir.length).replace(/^[\\/]+/, '') : path;
}

export function eta(task) {
    const rest = Number(task.totalLength) - Number(task.completedLength);
    const s = Number(task.downloadSpeed);
    return s > 0 ? rest / s : Infinity;
}

export function hexToBits(hex) {
    const bits = [];
    for (const c of hex || '') {
        const v = parseInt(c, 16);
        for (let i = 3; i >= 0; i--) bits.push((v >> i) & 1);
    }
    return bits;
}
