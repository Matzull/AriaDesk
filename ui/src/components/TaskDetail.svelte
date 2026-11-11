<script>
    import { onMount } from 'svelte';
    import Icon from './Icon.svelte';
    import Progress from './Progress.svelte';
    import OptionField from './OptionField.svelte';
    import { rpc, pause, resume, remove, retry, canRetry, toast, refresh } from '../lib/store.svelte.js';
    import { settings } from '../lib/settings.svelte.js';
    import { back, go } from '../lib/route.svelte.js';
    import { bytes, speed, duration, percent, taskName, eta, relativePath, hexToBits } from '../lib/format.js';
    import { taskOptions, taskOptionAllowed, toDisplay, toSubmit, validate, aria2Errors } from '../lib/options.js';
    import { isDesktop, send } from '../lib/native.js';
    import { t } from '../lib/i18n.js';

    let { gid, tab } = $props();

    let task = $state(null);
    let missing = $state(false);
    let peers = $state([]);
    let servers = $state([]);
    let fileSel = $state(null); // edited BT file selection, null when unchanged
    let options = $state(null);
    let edited = $state({});

    const isBt = $derived(!!task?.bittorrent);
    const stopped = $derived(task && ['complete', 'error', 'removed'].includes(task.status));
    const tabs = $derived(['overview', 'files', ...(isBt ? ['peers', 'trackers'] : ['sources']), 'options']);

    async function load() {
        try {
            const next = await rpc('tellStatus', gid);
            task = next;
            if (next.status === 'active') {
                if (next.bittorrent && tab === 'peers') peers = await rpc('getPeers', gid);
                if (!next.bittorrent && tab === 'sources') servers = await rpc('getServers', gid);
            }
        } catch (e) {
            if (/not found/i.test(e.message)) missing = true;
        }
    }

    async function loadOptions() {
        try {
            options = await rpc('getOption', gid);
            edited = {};
        } catch (e) {
            toast(e.message, 'error');
        }
    }

    onMount(() => {
        load();
        const timer = setInterval(load, Math.max(1000, settings.refreshInterval));
        return () => clearInterval(timer);
    });

    $effect(() => {
        if (tab === 'options' && !options) loadOptions();
        if (tab === 'peers' || tab === 'sources') load();
    });

    function setTab(next) {
        go(`/task/${gid}/${next}`);
    }

    const shownOptions = $derived(task ? taskOptions.filter((o) => (o.category !== 'bittorrent' || isBt) && (stopped || taskOptionAllowed(o, task.status, 'show'))) : []);
    const canEdit = (o) => !stopped && taskOptionAllowed(o, task.status, 'update');
    const dirty = $derived(Object.keys(edited).filter((k) => edited[k] !== toDisplay(k, options?.[k])));

    async function saveOptions() {
        const invalid = dirty.find((k) => !validate(k, edited[k]));
        if (invalid) return toast(t('invalidValue', { name: invalid }), 'error');
        const changes = Object.fromEntries(dirty.map((k) => [k, toSubmit(k, edited[k])]));
        try {
            await rpc('changeOption', gid, changes);
            toast(t('saved'), 'ok');
            loadOptions();
        } catch (e) {
            toast(e.message, 'error');
        }
    }

    const selection = $derived(fileSel ?? task?.files.map((f) => f.selected === 'true') ?? []);

    function toggleFile(i) {
        const next = [...selection];
        next[i] = !next[i];
        fileSel = next;
    }

    async function saveFiles() {
        const indexes = selection.map((on, i) => (on ? i + 1 : 0)).filter(Boolean);
        if (!indexes.length) return toast(t('selectOneFile'), 'error');
        try {
            await rpc('changeOption', gid, { 'select-file': indexes.join(',') });
            fileSel = null;
            toast(t('saved'), 'ok');
            load();
        } catch (e) {
            toast(e.message, 'error');
        }
    }

    function openFolder(file) {
        send('open-dir', task.dir, file ? relativePath(task, file.path) : '');
    }

    function copy(text) {
        navigator.clipboard.writeText(text).then(() => toast(t('copied'), 'ok'));
    }

    async function removeTask() {
        if (settings.confirmRemove && !confirm(t('confirmRemove', { n: 1 }))) return;
        await remove([task]);
        back();
    }

    // pieces, grouped so the map has at most 300 cells
    const pieces = $derived.by(() => {
        if (!task?.bitfield) return [];
        const bits = hexToBits(task.bitfield).slice(0, Number(task.numPieces));
        const size = Math.max(1, Math.ceil(bits.length / 300));
        const cells = [];
        for (let i = 0; i < bits.length; i += size) {
            const group = bits.slice(i, i + size);
            cells.push(group.reduce((a, b) => a + b, 0) / group.length);
        }
        return cells;
    });

    const clients = { qB: 'qBittorrent', TR: 'Transmission', UT: 'µTorrent', UM: 'µTorrent Mac', DE: 'Deluge', LT: 'libtorrent', lt: 'rTorrent', A2: 'aria2', BC: 'BitComet', AZ: 'Vuze', BI: 'BiglyBT', KT: 'KTorrent', FD: 'Free Download Manager', XL: 'Xunlei', SD: 'Thunder', TIX: 'Tixati', WW: 'WebTorrent', FW: 'FrostWire', PI: 'PicoTorrent' };
    function client(peerId) {
        let id = '';
        try {
            id = decodeURIComponent(peerId);
        } catch {
            return '';
        }
        const m = id.match(/^-([A-Za-z]{2})(\w{4})-/);
        if (m) return `${clients[m[1]] || m[1]} ${m[2].replace(/(\w)(?=\w)/g, '$1.').replace(/(\.0)+$/, '')}`;
        if (id.startsWith('A2-')) return 'aria2';
        return '';
    }
    const peerProgress = (peer) => (task.numPieces ? (hexToBits(peer.bitfield).filter(Boolean).length / Number(task.numPieces)) * 100 : 0);
</script>

<aside class="detail">
    <header>
        <button class="icon ghost" onclick={back} title={t('close')}><Icon name="x" /></button>
        <h2 title={task ? taskName(task) : gid}>{task ? taskName(task) : gid}</h2>
        {#if task}
            {#if task.status === 'paused'}
                <button class="icon" title={t('resume')} onclick={() => resume([task]).then(load)}><Icon name="play" size={16} /></button>
            {:else if !stopped}
                <button class="icon" title={t('pause')} onclick={() => pause([task]).then(load)}><Icon name="pause" size={16} /></button>
            {:else if canRetry(task)}
                <button class="icon" title={t('retry')} onclick={() => retry([task]).then(back)}><Icon name="retry" size={16} /></button>
            {/if}
            {#if isDesktop}
                <button class="icon" title={t('openFolder')} onclick={() => openFolder(task.files.length === 1 ? task.files[0] : null)}><Icon name="folder" size={16} /></button>
            {/if}
            <button class="icon danger" title={t('remove')} onclick={removeTask}><Icon name="trash" size={16} /></button>
        {/if}
    </header>

    {#if missing}
        <div class="empty">{t('taskGone')}</div>
    {:else if task}
        <nav class="tabs">
            {#each tabs as id}
                <button class:current={tab === id} onclick={() => setTab(id)}>{t('tab.' + id)}</button>
            {/each}
        </nav>

        <div class="body">
            {#if tab === 'overview'}
                {@const pct = percent(task.completedLength, task.totalLength)}
                <div class="hero">
                    <div class="pct">{pct.toFixed(1)}<small>%</small></div>
                    <div class="hero-side">
                        <span class="badge {task.status}">{t('status.' + task.status)}</span>
                        {#if task.status === 'active'}
                            <span class="dl">↓ {speed(task.downloadSpeed)}</span>
                            {#if isBt}<span class="ul">↑ {speed(task.uploadSpeed)}</span>{/if}
                        {/if}
                    </div>
                </div>
                <Progress value={pct} status={task.status} height={8} />

                {#if task.status === 'error'}
                    <div class="error-box">
                        <Icon name="alert" />
                        <div>
                            <b>{aria2Errors[task.errorCode]?.[settings.language === 'es' ? 1 : 0] || t('status.error')}</b>
                            <div>{task.errorMessage}</div>
                        </div>
                    </div>
                {/if}

                <dl class="grid">
                    <dt>{t('size')}</dt><dd>{bytes(task.totalLength)}</dd>
                    <dt>{t('downloaded')}</dt><dd>{bytes(task.completedLength)}</dd>
                    {#if isBt}<dt>{t('uploaded')}</dt><dd>{bytes(task.uploadLength)}{Number(task.completedLength) ? ` · ${t('ratio')} ${(task.uploadLength / task.completedLength).toFixed(2)}` : ''}</dd>{/if}
                    {#if task.status === 'active'}
                        <dt>{t('remaining')}</dt><dd>{duration(eta(task))}</dd>
                        <dt>{t('connectionsTitle')}</dt><dd>{task.connections}{isBt ? ` · ${task.numSeeders} ${t('seeders')}` : ''}</dd>
                    {/if}
                    <dt>{t('directory')}</dt>
                    <dd class="path">
                        {task.dir}
                        {#if isDesktop}<button class="link" onclick={() => openFolder(null)}>{t('open')}</button>{/if}
                    </dd>
                    {#if Number(task.numPieces)}<dt>{t('pieces')}</dt><dd>{task.numPieces} × {bytes(task.pieceLength)}</dd>{/if}
                    <dt>GID</dt><dd class="mono">{task.gid}</dd>
                    {#if task.infoHash}
                        <dt>{t('infoHash')}</dt>
                        <dd class="mono">{task.infoHash} <button class="link" onclick={() => copy(`magnet:?xt=urn:btih:${task.infoHash}`)}>{t('copyMagnet')}</button></dd>
                    {/if}
                    {#if task.bittorrent?.comment}<dt>{t('comment')}</dt><dd>{task.bittorrent.comment}</dd>{/if}
                    {#if task.bittorrent?.creationDate}<dt>{t('created')}</dt><dd>{new Date(task.bittorrent.creationDate * 1000).toLocaleString(settings.language)}</dd>{/if}
                </dl>

                {#if pieces.length}
                    <h3>{t('pieceMap')}</h3>
                    <div class="pieces">
                        {#each pieces as p}<i style:opacity={0.15 + p * 0.85}></i>{/each}
                    </div>
                {/if}
            {:else if tab === 'files'}
                <div class="files">
                    {#each task.files as file, i}
                        {@const fp = percent(file.completedLength, file.length)}
                        <div class="file">
                            {#if isBt && task.files.length > 1}
                                <input type="checkbox" checked={selection[i]} disabled={stopped} onchange={() => toggleFile(i)} />
                            {/if}
                            <div class="main">
                                <div class="name">{relativePath(task, file.path) || file.uris?.[0]?.uri || '—'}</div>
                                <Progress value={fp} status={file.selected === 'true' ? task.status : 'paused'} height={3} />
                            </div>
                            <span class="size">{bytes(file.length)} · {fp.toFixed(0)}%</span>
                            {#if isDesktop && file.path}
                                <button class="icon ghost" title={t('openFolder')} onclick={() => openFolder(file)}><Icon name="folder" size={15} /></button>
                            {/if}
                        </div>
                    {/each}
                </div>
                {#if fileSel}
                    <div class="savebar">
                        <button onclick={() => (fileSel = null)}>{t('cancel')}</button>
                        <button class="primary" onclick={saveFiles}>{t('applySelection')}</button>
                    </div>
                {/if}
            {:else if tab === 'peers'}
                {#if task.status !== 'active'}
                    <div class="empty">{t('onlyActive')}</div>
                {:else}
                    <table>
                        <thead><tr><th>{t('address')}</th><th>{t('client')}</th><th>%</th><th>↓</th><th>↑</th></tr></thead>
                        <tbody>
                            {#each peers as peer}
                                <tr>
                                    <td class="mono">{peer.ip.includes(':') ? `[${peer.ip}]` : peer.ip}:{peer.port}{peer.seeder === 'true' ? ' 🌱' : ''}</td>
                                    <td>{client(peer.peerId)}</td>
                                    <td>{peerProgress(peer).toFixed(0)}</td>
                                    <td class="dl">{speed(peer.downloadSpeed)}</td>
                                    <td class="ul">{speed(peer.uploadSpeed)}</td>
                                </tr>
                            {:else}
                                <tr><td colspan="5" class="muted">{t('noPeers')}</td></tr>
                            {/each}
                        </tbody>
                    </table>
                {/if}
            {:else if tab === 'trackers'}
                <ul class="plain">
                    {#each (task.bittorrent.announceList || []).flat() as tracker}
                        <li class="mono">{tracker}</li>
                    {:else}
                        <li class="muted">{t('noTrackers')}</li>
                    {/each}
                </ul>
            {:else if tab === 'sources'}
                {#each task.files as file}
                    <ul class="plain">
                        {#each file.uris || [] as uri}
                            {@const server = servers.flatMap((s) => s.servers).find((s) => s.uri === uri.uri)}
                            <li>
                                <span class="badge {uri.status === 'used' ? 'active' : 'waiting'}">{t('uri.' + uri.status)}</span>
                                <span class="mono">{uri.uri}</span>
                                {#if server}<span class="dl">↓ {speed(server.downloadSpeed)}</span>{/if}
                            </li>
                        {/each}
                    </ul>
                {/each}
            {:else if tab === 'options'}
                {#if options}
                    {#each shownOptions as o (o.key)}
                        <OptionField
                            compact
                            key={o.key}
                            bind:value={() => edited[o.key] ?? toDisplay(o.key, options[o.key]), (v) => (edited[o.key] = v)}
                            readonly={!canEdit(o)}
                            changed={dirty.includes(o.key)}
                        />
                    {/each}
                    {#if dirty.length}
                        <div class="savebar">
                            <button onclick={() => (edited = {})}>{t('cancel')}</button>
                            <button class="primary" onclick={saveOptions}>{t('saveChanges')}</button>
                        </div>
                    {/if}
                {:else}
                    <div class="empty">{t('loading')}</div>
                {/if}
            {/if}
        </div>
    {:else}
        <div class="empty">{t('loading')}</div>
    {/if}
</aside>

<style>
    .detail {
        width: min(52%, 640px);
        flex: none;
        display: flex;
        flex-direction: column;
        border-left: 1px solid var(--border);
        background: var(--surface);
        animation: enter 0.16s ease-out;
    }
    @keyframes enter {
        from {
            transform: translateX(20px);
            opacity: 0;
        }
    }
    header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 12px 14px;
    }
    h2 {
        flex: 1;
        margin: 0;
        font-size: 15px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .tabs {
        display: flex;
        gap: 2px;
        padding: 0 14px;
        border-bottom: 1px solid var(--border);
    }
    .tabs button {
        border: none;
        border-radius: 0;
        background: none;
        padding: 9px 10px;
        color: var(--muted);
        border-bottom: 2px solid transparent;
    }
    .tabs button.current {
        color: var(--text);
        border-bottom-color: var(--accent);
    }
    .body {
        flex: 1;
        overflow-y: auto;
        padding: 16px 18px 70px;
        position: relative;
    }
    .hero {
        display: flex;
        align-items: end;
        justify-content: space-between;
        margin-bottom: 10px;
    }
    .pct {
        font-size: 38px;
        font-weight: 700;
        line-height: 1;
        font-variant-numeric: tabular-nums;
    }
    .pct small {
        font-size: 18px;
        color: var(--muted);
    }
    .hero-side {
        display: flex;
        gap: 10px;
        align-items: center;
        font-variant-numeric: tabular-nums;
    }
    .grid {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 8px 18px;
        margin: 20px 0;
    }
    dt {
        color: var(--muted);
    }
    dd {
        margin: 0;
        min-width: 0;
        overflow-wrap: anywhere;
    }
    .mono {
        font-family: var(--mono);
        font-size: 12px;
        overflow-wrap: anywhere;
    }
    .link {
        border: none;
        background: none;
        padding: 0 4px;
        color: var(--accent);
        display: inline;
    }
    h3 {
        font-size: 13px;
        color: var(--muted);
        font-weight: 600;
        margin: 0 0 8px;
    }
    .pieces {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(9px, 1fr));
        gap: 2px;
    }
    .pieces i {
        height: 9px;
        border-radius: 2px;
        background: var(--accent);
    }
    .error-box {
        display: flex;
        gap: 10px;
        margin-top: 16px;
        padding: 12px;
        border-radius: 10px;
        color: var(--danger);
        background: color-mix(in srgb, var(--danger) 10%, transparent);
    }
    .error-box div div {
        color: var(--text-2);
        font-size: 12px;
        margin-top: 2px;
    }
    .file {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 0;
        border-bottom: 1px solid var(--border);
    }
    .file .main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 5px;
    }
    .file .name {
        font-size: 13px;
        overflow-wrap: anywhere;
    }
    .size {
        font-size: 12px;
        color: var(--muted);
        white-space: nowrap;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        font-size: 12px;
    }
    th {
        text-align: left;
        color: var(--muted);
        font-weight: 500;
        padding: 6px 4px;
        border-bottom: 1px solid var(--border);
    }
    td {
        padding: 6px 4px;
        border-bottom: 1px solid var(--border);
        font-variant-numeric: tabular-nums;
    }
    .plain {
        list-style: none;
        margin: 0 0 12px;
        padding: 0;
    }
    .plain li {
        display: flex;
        gap: 8px;
        align-items: center;
        padding: 7px 0;
        border-bottom: 1px solid var(--border);
    }
    .dl {
        color: var(--down);
    }
    .ul {
        color: var(--up);
    }
    .savebar {
        position: sticky;
        bottom: -60px;
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        padding: 12px 0;
        background: var(--surface);
    }
    .empty {
        padding: 40px;
        text-align: center;
        color: var(--muted);
    }
    @media (max-width: 980px) {
        .detail {
            position: fixed;
            right: 0;
            top: 0;
            bottom: 0;
            width: min(92vw, 560px);
            box-shadow: var(--shadow);
            z-index: 20;
        }
    }
</style>
