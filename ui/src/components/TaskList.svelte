<script>
    import Icon from './Icon.svelte';
    import Progress from './Progress.svelte';
    import { app, pause, resume, remove, retry, canRetry, command } from '../lib/store.svelte.js';
    import { settings } from '../lib/settings.svelte.js';
    import { go } from '../lib/route.svelte.js';
    import { bytes, speed, duration, percent, taskName, eta } from '../lib/format.js';
    import { isDesktop, send } from '../lib/native.js';
    import { aria2Errors } from '../lib/options.js';
    import { t } from '../lib/i18n.js';

    let { filter, selectedGid } = $props();

    let query = $state('');
    let selection = $state(new Set());

    const sorters = {
        added: null,
        name: (a, b) => taskName(a).localeCompare(taskName(b), undefined, { numeric: true }),
        size: (a, b) => a.totalLength - b.totalLength,
        progress: (a, b) => percent(a.completedLength, a.totalLength) - percent(b.completedLength, b.totalLength),
        speed: (a, b) => a.downloadSpeed - b.downloadSpeed,
        remaining: (a, b) => eta(a) - eta(b)
    };

    const tasks = $derived.by(() => {
        let list = { active: app.active, waiting: app.waiting, stopped: app.stopped }[filter] || [...app.active, ...app.waiting, ...app.stopped];
        const q = query.trim().toLowerCase();
        if (q) list = list.filter((task) => taskName(task).toLowerCase().includes(q));
        const sorter = sorters[settings.sort.key];
        if (sorter) {
            list = [...list].sort((a, b) => sorter(a, b) || 0);
            if (settings.sort.desc) list.reverse();
        }
        return list;
    });

    const selected = $derived(tasks.filter((task) => selection.has(task.gid)));
    const allSelected = $derived(tasks.length > 0 && selected.length === tasks.length);
    const movable = $derived(selected.length === 1 && selected[0].status !== 'active' && app.waiting.includes(selected[0]));

    $effect(() => {
        filter;
        selection = new Set();
    });

    function toggle(gid) {
        const next = new Set(selection);
        next.has(gid) ? next.delete(gid) : next.add(gid);
        selection = next;
    }

    function toggleAll() {
        selection = allSelected ? new Set() : new Set(tasks.map((task) => task.gid));
    }

    async function removeSelected(list = selected) {
        if (!list.length) return;
        if (settings.confirmRemove && !confirm(t('confirmRemove', { n: list.length }))) return;
        await remove(list);
        selection = new Set();
    }

    function move(task, delta) {
        command('changePosition', task.gid, delta, 'POS_CUR');
    }

    function onKey(e) {
        if (/^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName)) return;
        if (e.key === 'Delete' && selected.length) removeSelected();
        if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
            e.preventDefault();
            selection = new Set(tasks.map((task) => task.gid));
        }
    }

    function statusLine(task) {
        const done = bytes(task.completedLength);
        const total = Number(task.totalLength) ? bytes(task.totalLength) : '?';
        switch (task.status) {
            case 'active':
                if (task.seeder === 'true' || (Number(task.totalLength) && task.completedLength === task.totalLength)) {
                    return `${t('status.seeding')} · ${total} · ${t('uploaded')} ${bytes(task.uploadLength)}`;
                }
                return `${done} / ${total} · ${t('remaining')} ${duration(eta(task))} · ${task.connections} ${t('connections')}`;
            case 'complete':
                return total;
            case 'error':
                return aria2Errors[task.errorCode]?.[settings.language === 'es' ? 1 : 0] || task.errorMessage || t('status.error');
            default:
                return `${done} / ${total}`;
        }
    }

    const statusIcon = { active: 'down', waiting: 'clock', paused: 'pause', complete: 'done', error: 'alert', removed: 'x' };
</script>

<svelte:window onkeydown={onKey} />

<section class="list">
    <header>
        <h1>{t('filter.' + filter)}</h1>
        <label class="search">
            <Icon name="search" size={16} />
            <input type="search" placeholder={t('search')} bind:value={query} />
        </label>
        <label class="sort" title={t('sortBy')}>
            <Icon name="sort" size={16} />
            <select bind:value={settings.sort.key}>
                {#each Object.keys(sorters) as key}
                    <option value={key}>{t('sort.' + key)}</option>
                {/each}
            </select>
            <button class="icon ghost" onclick={() => (settings.sort.desc = !settings.sort.desc)} title={t('reverse')}>
                <Icon name={settings.sort.desc ? 'chevronDown' : 'chevronUp'} size={16} />
            </button>
        </label>
    </header>

    <div class="toolbar">
        <input type="checkbox" checked={allSelected} indeterminate={selected.length > 0 && !allSelected} onchange={toggleAll} aria-label={t('selectAll')} />
        {#if selected.length}
            <span class="muted">{t('nSelected', { n: selected.length })}</span>
            <button onclick={() => resume(selected)}><Icon name="play" size={16} />{t('resume')}</button>
            <button onclick={() => pause(selected)}><Icon name="pause" size={16} />{t('pause')}</button>
            {#if selected.some(canRetry)}
                <button onclick={() => retry(selected.filter(canRetry))}><Icon name="retry" size={16} />{t('retry')}</button>
            {/if}
            {#if movable}
                <button class="icon" onclick={() => move(selected[0], -1)} title={t('moveUp')}><Icon name="chevronUp" size={16} /></button>
                <button class="icon" onclick={() => move(selected[0], 1)} title={t('moveDown')}><Icon name="chevronDown" size={16} /></button>
            {/if}
            <button class="danger" onclick={() => removeSelected()}><Icon name="trash" size={16} />{t('remove')}</button>
        {:else}
            <button onclick={() => command('unpauseAll')}><Icon name="play" size={16} />{t('resumeAll')}</button>
            <button onclick={() => command('pauseAll')}><Icon name="pause" size={16} />{t('pauseAll')}</button>
            {#if filter === 'stopped' && app.stopped.length}
                <button class="danger" onclick={() => confirm(t('confirmPurge')) && command('purgeDownloadResult')}>
                    <Icon name="trash" size={16} />{t('clearFinished')}
                </button>
            {/if}
        {/if}
    </div>

    <div class="rows">
        {#each tasks as task (task.gid)}
            {@const pct = percent(task.completedLength, task.totalLength)}
            <div
                class="row {task.status}"
                class:selected={selection.has(task.gid)}
                class:open={selectedGid === task.gid}
                role="button"
                tabindex="0"
                onclick={() => go(`/task/${task.gid}`)}
                onkeydown={(e) => e.key === 'Enter' && go(`/task/${task.gid}`)}
            >
                <input type="checkbox" checked={selection.has(task.gid)} onclick={(e) => e.stopPropagation()} onchange={() => toggle(task.gid)} aria-label={taskName(task)} />
                <span class="state" title={t('status.' + task.status)}><Icon name={statusIcon[task.status]} size={18} /></span>
                <div class="main">
                    <div class="name" title={taskName(task)}>{taskName(task)}</div>
                    <Progress value={pct} status={task.status} />
                    <div class="sub">{statusLine(task)}</div>
                </div>
                <div class="rates">
                    {#if task.status === 'active'}
                        <span class="dl">↓ {speed(task.downloadSpeed)}</span>
                        {#if task.bittorrent}<span class="ul">↑ {speed(task.uploadSpeed)}</span>{/if}
                    {:else}
                        <span class="muted">{pct.toFixed(pct === 100 || pct === 0 ? 0 : 1)}%</span>
                    {/if}
                </div>
                <div class="actions">
                    {#if task.status === 'paused'}
                        <button class="icon ghost" title={t('resume')} onclick={(e) => (e.stopPropagation(), resume([task]))}><Icon name="play" size={16} /></button>
                    {:else if task.status === 'active' || task.status === 'waiting'}
                        <button class="icon ghost" title={t('pause')} onclick={(e) => (e.stopPropagation(), pause([task]))}><Icon name="pause" size={16} /></button>
                    {:else if canRetry(task)}
                        <button class="icon ghost" title={t('retry')} onclick={(e) => (e.stopPropagation(), retry([task]))}><Icon name="retry" size={16} /></button>
                    {/if}
                    {#if isDesktop && task.status === 'complete'}
                        <button class="icon ghost" title={t('openFolder')} onclick={(e) => (e.stopPropagation(), send('open-dir', task.dir, task.files?.length === 1 ? task.files[0].path.slice(task.dir.length).replace(/^[\\/]+/, '') : ''))}>
                            <Icon name="folder" size={16} />
                        </button>
                    {/if}
                    <button class="icon ghost" title={t('remove')} onclick={(e) => (e.stopPropagation(), removeSelected([task]))}><Icon name="trash" size={16} /></button>
                </div>
            </div>
        {:else}
            <div class="empty">
                {#if app.status === 'error'}
                    <Icon name="alert" size={40} />
                    <p>{t('cannotConnect')}</p>
                    <small>{app.error}</small>
                    <a class="btn" href="#/settings/connection">{t('connectionSettings')}</a>
                {:else if query}
                    <Icon name="search" size={40} />
                    <p>{t('noMatches')}</p>
                {:else}
                    <Icon name="down" size={40} />
                    <p>{t('noDownloads')}</p>
                    <small>{t('noDownloadsHint')}</small>
                {/if}
            </div>
        {/each}
    </div>
</section>

<style>
    .list {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
    }
    header {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 8px 12px;
        padding: 16px 20px 8px;
    }
    h1 {
        margin: 0;
        font-size: 20px;
        flex: 1 0 auto;
        white-space: nowrap;
    }
    .search,
    .sort {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 0 4px 0 10px;
        border: 1px solid var(--border);
        border-radius: 8px;
        background: var(--surface);
        color: var(--muted);
    }
    .search input,
    .sort select {
        border: none;
        background: transparent;
        padding: 7px 4px;
        box-shadow: none;
    }
    .search input {
        width: 180px;
    }
    .sort select {
        appearance: none;
        cursor: pointer;
    }
    .toolbar {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 6px 20px 10px 30px;
        border-bottom: 1px solid var(--border);
        min-height: 48px;
        flex-wrap: wrap;
    }
    .toolbar > input {
        margin-right: 8px;
    }
    .toolbar .muted {
        margin-right: 6px;
    }
    .rows {
        flex: 1;
        overflow-y: auto;
        padding: 8px 12px 24px;
    }
    .row {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 12px 10px 18px;
        border-radius: 10px;
        cursor: pointer;
        border: 1px solid transparent;
    }
    .row:hover {
        background: var(--hover);
    }
    .row.selected {
        background: var(--selected);
    }
    .row.open {
        border-color: var(--accent);
        background: var(--selected);
    }
    .state {
        color: var(--muted);
    }
    .active .state {
        color: var(--down);
    }
    .complete .state {
        color: var(--ok);
    }
    .error .state {
        color: var(--danger);
    }
    .main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 5px;
    }
    .name {
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .sub {
        font-size: 12px;
        color: var(--muted);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .error .sub {
        color: var(--danger);
    }
    .rates {
        width: 96px;
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        font-size: 12px;
        font-variant-numeric: tabular-nums;
    }
    .dl {
        color: var(--down);
    }
    .ul {
        color: var(--up);
    }
    .actions {
        display: flex;
        gap: 2px;
        opacity: 0;
    }
    .row:hover .actions,
    .row:focus-within .actions {
        opacity: 1;
    }
    .empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 6px;
        padding: 80px 20px;
        color: var(--muted);
        text-align: center;
    }
    .empty p {
        margin: 8px 0 0;
        font-size: 16px;
        color: var(--text-2);
    }
    .empty .btn {
        margin-top: 12px;
    }
    @media (max-width: 1100px) {
        .search input {
            width: 120px;
        }
    }
</style>
