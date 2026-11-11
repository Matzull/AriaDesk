// Hash based routing: #/downloads/<filter>, #/task/<gid>/<tab>, #/new, #/settings/<section>.
// The desktop backend reads the fragment to know whether the new task page is open.

import { boot } from './native.js';

const parse = () => (location.hash.replace(/^#!?/, '') || '/downloads/all').split('/').filter(Boolean);

if (boot.initialRoute && !location.hash) {
    location.hash = boot.initialRoute;
}

const initial = parse();
export const route = $state({
    parts: initial,
    lastFilter: initial[0] === 'downloads' ? initial[1] || 'all' : 'all',
    previous: '/downloads/all'
});

window.addEventListener('hashchange', () => {
    const parts = parse();
    if (route.parts[0] !== 'new') {
        route.previous = '/' + route.parts.join('/');
    }
    if (parts[0] === 'downloads') {
        route.lastFilter = parts[1] || 'all';
    }
    route.parts = parts;
});

export function go(path) {
    location.hash = path;
}

/** Leaves an overlay (new task, task detail) for the list it was opened from. */
export function back() {
    const target = route.parts[0] === 'task' ? '/downloads/' + route.lastFilter : route.previous;
    go(target.startsWith('/new') ? '/downloads/' + route.lastFilter : target);
}
