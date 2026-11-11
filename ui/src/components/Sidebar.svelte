<script>
    import Icon from './Icon.svelte';
    import SpeedChart from './SpeedChart.svelte';
    import { app } from '../lib/store.svelte.js';
    import { settings } from '../lib/settings.svelte.js';
    import { route, go } from '../lib/route.svelte.js';
    import { speed } from '../lib/format.js';
    import { t } from '../lib/i18n.js';

    let { filter } = $props();

    const paused = $derived(app.waiting.filter((x) => x.status === 'paused').length);
    const items = $derived([
        { id: 'all', icon: 'layers', count: app.active.length + app.waiting.length + app.stopped.length },
        { id: 'active', icon: 'down', count: app.active.length },
        { id: 'waiting', icon: 'clock', count: app.waiting.length, extra: paused ? `${paused} ${t('pausedShort')}` : '' },
        { id: 'stopped', icon: 'done', count: app.stopped.length }
    ]);
</script>

<aside>
    <div class="brand">
        <img src="icon.png" alt="" width="26" height="26" />
        <span>AriaDesk</span>
    </div>

    <button class="add" onclick={() => go('/new')} title="Ctrl+N">
        <Icon name="plus" />
        {t('addDownload')}
    </button>

    <nav>
        {#each items as item (item.id)}
            <a href="#/downloads/{item.id}" class:current={filter === item.id}>
                <Icon name={item.icon} />
                <span class="label">{t('filter.' + item.id)}</span>
                {#if item.extra}<span class="extra">{item.extra}</span>{/if}
                <span class="count">{item.count}</span>
            </a>
        {/each}
        <div class="sep"></div>
        <a href="#/settings" class:current={route.parts[0] === 'settings'}>
            <Icon name="settings" />
            <span class="label">{t('settings')}</span>
        </a>
    </nav>

    <div class="footer">
        {#if settings.showChart}
            <SpeedChart history={app.history} />
        {/if}
        <div class="speeds">
            <span class="dl"><Icon name="down" size={14} />{speed(app.stat.downloadSpeed)}</span>
            <span class="ul"><Icon name="up" size={14} />{speed(app.stat.uploadSpeed)}</span>
        </div>
        <a class="conn {app.status}" href="#/settings/connection" title={app.error}>
            <i></i>
            {#if app.status === 'connected'}
                aria2 {app.version}
            {:else if app.status === 'error'}
                {t('disconnected')}
            {:else}
                {t('connecting')}
            {/if}
        </a>
    </div>
</aside>

<style>
    aside {
        width: 228px;
        flex: none;
        display: flex;
        flex-direction: column;
        background: var(--sidebar);
        border-right: 1px solid var(--border);
        padding: 14px 10px 10px;
        gap: 14px;
    }
    .brand {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 2px 8px;
        font-weight: 700;
        font-size: 16px;
        letter-spacing: 0.2px;
    }
    .add {
        justify-content: center;
        padding: 10px;
        background: var(--accent);
        color: var(--on-accent);
        border: none;
        font-weight: 600;
        border-radius: 10px;
    }
    .add:hover {
        background: var(--accent-strong);
    }
    nav {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    nav a {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 10px;
        border-radius: 8px;
        color: var(--text-2);
        text-decoration: none;
    }
    nav a:hover {
        background: var(--hover);
        color: var(--text);
    }
    nav a.current {
        background: var(--selected);
        color: var(--text);
        font-weight: 600;
    }
    .label {
        flex: 1;
    }
    .extra {
        font-size: 11px;
        color: var(--muted);
    }
    .count {
        font-size: 12px;
        min-width: 22px;
        text-align: center;
        padding: 1px 6px;
        border-radius: 99px;
        background: var(--chip);
        color: var(--text-2);
    }
    .sep {
        height: 1px;
        background: var(--border);
        margin: 8px 4px;
    }
    .footer {
        margin-top: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 10px;
        border-radius: 12px;
        background: var(--surface);
        border: 1px solid var(--border);
    }
    .speeds {
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        font-variant-numeric: tabular-nums;
    }
    .speeds span {
        display: flex;
        align-items: center;
        gap: 3px;
    }
    .dl {
        color: var(--down);
    }
    .ul {
        color: var(--up);
    }
    .conn {
        display: flex;
        align-items: center;
        gap: 7px;
        font-size: 12px;
        color: var(--muted);
        text-decoration: none;
    }
    .conn i {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: var(--warn);
    }
    .conn.connected i {
        background: var(--ok);
    }
    .conn.error i {
        background: var(--danger);
    }
</style>
