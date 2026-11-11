<script>
    import Icon from './Icon.svelte';
    import OptionField from './OptionField.svelte';
    import { app, connect, rpc, toast } from '../lib/store.svelte.js';
    import { settings, rpcUrl } from '../lib/settings.svelte.js';
    import { globalCategories, toDisplay, toSubmit, validate } from '../lib/options.js';
    import { boot, isDesktop, invoke, send, setNativeConfig, openExternal } from '../lib/native.js';
    import { Aria2 } from '../lib/rpc.js';
    import { optionLabel, t } from '../lib/i18n.js';

    let { section, devMode } = $props();

    const sections = [
        { id: 'connection', icon: 'server' },
        { id: 'interface', icon: 'monitor' },
        ...(isDesktop ? [{ id: 'desktop', icon: 'bolt' }] : []),
        { id: 'aria2', icon: 'sliders' },
        { id: 'about', icon: 'info' }
    ];

    // connection
    let rpcDraft = $state({ ...settings.rpc });
    let testing = $state(false);

    async function test() {
        testing = true;
        const client = new Aria2(rpcUrl(rpcDraft), rpcDraft.secret);
        try {
            const v = await client.call('getVersion');
            toast(t('connectedTo', { version: v.version }), 'ok');
        } catch (e) {
            toast(`${t('cannotConnect')}: ${e.message}`, 'error');
        } finally {
            client.close();
            testing = false;
            // the desktop backend keeps a single WebSocket, so the test replaced the main connection
            if (isDesktop && client.isWebSocket) connect();
        }
    }

    function saveConnection() {
        rpcDraft.port = Number(rpcDraft.port) || 6800;
        settings.rpc = { ...rpcDraft };
        connect();
    }

    // desktop
    const native = $state({ ...boot.nativeConfig });
    let magnet = $state(null);
    $effect(() => {
        if (section === 'desktop' && isDesktop && !magnet) invoke('magnet-status').then((r) => (magnet = r));
    });

    function setNative(key, value) {
        native[key] = value;
        setNativeConfig(key, value);
    }

    async function toggleMagnet(enabled) {
        magnet = await invoke('set-magnet-handler', enabled);
        if (enabled && !magnet.registered) toast(t('magnetFailed'), 'error');
    }

    // aria2 global options
    let category = $state('basicOptions');
    let filter = $state('');
    let globalOptions = $state(null);
    let edited = $state({});
    $effect(() => {
        if (section === 'aria2' && !globalOptions && app.status === 'connected') loadGlobal();
    });

    async function loadGlobal() {
        try {
            globalOptions = await rpc('getGlobalOption');
            edited = {};
        } catch (e) {
            toast(e.message, 'error');
        }
    }

    const shownKeys = $derived.by(() => {
        const q = filter.trim().toLowerCase();
        if (!q) return globalCategories[category];
        const all = [...new Set(Object.values(globalCategories).flat())];
        return all.filter((k) => k.includes(q) || optionLabel(k).toLowerCase().includes(q));
    });
    const dirty = $derived(Object.keys(edited).filter((k) => edited[k] !== toDisplay(k, globalOptions?.[k])));

    async function saveGlobal() {
        const invalid = dirty.find((k) => !validate(k, edited[k]));
        if (invalid) return toast(t('invalidValue', { name: invalid }), 'error');
        try {
            await rpc('changeGlobalOption', Object.fromEntries(dirty.map((k) => [k, toSubmit(k, edited[k])])));
            toast(t('saved'), 'ok');
            loadGlobal();
        } catch (e) {
            toast(e.message, 'error');
        }
    }

    // about
    let startupOutput = $state(null);

    async function aria2Action(method, question) {
        if (question && !confirm(question)) return;
        try {
            await rpc(method);
            toast(t('done'), 'ok');
        } catch (e) {
            toast(e.message, 'error');
        }
    }
</script>

<div class="settings">
    <nav>
        <h1>{t('settings')}</h1>
        {#each sections as s}
            <a href="#/settings/{s.id}" class:current={section === s.id}><Icon name={s.icon} size={17} />{t('section.' + s.id)}</a>
        {/each}
    </nav>

    <div class="content">
        {#if section === 'connection'}
            <h2>{t('section.connection')}</h2>
            <p class="lead">{t('connectionLead')}</p>
            <div class="form">
                <label>{t('protocol')}
                    <select bind:value={rpcDraft.protocol}>
                        <option value="http">HTTP</option>
                        <option value="https">HTTPS</option>
                        <option value="ws">WebSocket</option>
                        <option value="wss">WebSocket (TLS)</option>
                    </select>
                </label>
                <label>{t('host')}<input bind:value={rpcDraft.host} spellcheck="false" /></label>
                <label>{t('port')}<input bind:value={rpcDraft.port} inputmode="numeric" /></label>
                <label>{t('path')}<input bind:value={rpcDraft.path} spellcheck="false" /></label>
                <label class="wide">{t('secret')}<input type="password" bind:value={rpcDraft.secret} autocomplete="off" /></label>
            </div>
            <p class="muted mono">{rpcUrl(rpcDraft)}</p>
            <div class="buttons">
                <button onclick={test} disabled={testing}>{t('testConnection')}</button>
                <button class="primary" onclick={saveConnection}>{t('saveAndConnect')}</button>
            </div>
            {#if !isDesktop}<p class="note">{t('webCorsNote')}</p>{/if}
        {:else if section === 'interface'}
            <h2>{t('section.interface')}</h2>
            <div class="rows">
                <label class="row">{t('language')}
                    <select bind:value={settings.language}>
                        <option value="en">English</option>
                        <option value="es">Español</option>
                    </select>
                </label>
                <label class="row">{t('theme')}
                    <select bind:value={settings.theme}>
                        <option value="system">{t('theme.system')}</option>
                        <option value="light">{t('theme.light')}</option>
                        <option value="dark">{t('theme.dark')}</option>
                    </select>
                </label>
                <label class="row">{t('refreshInterval')}
                    <select bind:value={settings.refreshInterval}>
                        {#each [500, 1000, 2000, 3000, 5000, 10000] as ms}<option value={ms}>{ms / 1000} s</option>{/each}
                    </select>
                </label>
                <label class="row">{t('notifications')}
                    <input
                        type="checkbox"
                        class="switch"
                        bind:checked={settings.notifications}
                        onchange={() => !isDesktop && settings.notifications && window.Notification?.requestPermission()}
                    />
                </label>
                <label class="row">{t('confirmRemoveSetting')}<input type="checkbox" class="switch" bind:checked={settings.confirmRemove} /></label>
                <label class="row">{t('showChart')}<input type="checkbox" class="switch" bind:checked={settings.showChart} /></label>
            </div>
        {:else if section === 'desktop'}
            <h2>{t('section.desktop')}</h2>
            <div class="rows">
                <label class="row">{t('windowPosition')}
                    <select value={native.defaultPosition || 'last-position'} onchange={(e) => setNative('defaultPosition', e.target.value)}>
                        <option value="last-position">{t('position.last')}</option>
                        <option value="screen-center">{t('position.center')}</option>
                    </select>
                </label>
                <label class="row">{boot.os === 'macos' ? t('closeHides') : t('closeToTray')}
                    <input type="checkbox" class="switch" checked={native.minimizedToTray} onchange={(e) => setNative('minimizedToTray', e.target.checked)} />
                </label>
                <label class="row">{t('magnetHandler')}
                    <input type="checkbox" class="switch" checked={!!magnet?.enabled} disabled={!magnet} onchange={(e) => toggleMagnet(e.target.checked)} />
                </label>
                <label class="row">{t('aria2Always')}
                    <input type="checkbox" class="switch" checked={native.autoStartAria2} onchange={(e) => setNative('autoStartAria2', e.target.checked)} />
                </label>
            </div>
            <h3>{t('startupCommand')}</h3>
            <p class="lead">{t('startupCommandLead')}</p>
            <div class="form">
                <label class="wide">{t('command')}
                    <input value={native.execCommandOnStartup || ''} placeholder="aria2c" spellcheck="false" onchange={(e) => setNative('execCommandOnStartup', e.target.value)} />
                </label>
                <label class="wide">{t('arguments')}
                    <input value={native.execCommandArgumentsOnStartup || ''} placeholder="--enable-rpc --rpc-listen-all=false" spellcheck="false" onchange={(e) => setNative('execCommandArgumentsOnStartup', e.target.value)} />
                </label>
            </div>
            <label class="row">{t('detached')}
                <input type="checkbox" class="switch" checked={native.execDetachedCommandOnStartup} onchange={(e) => setNative('execDetachedCommandOnStartup', e.target.checked)} />
            </label>
            {#if devMode}
                <div class="buttons">
                    <button onclick={async () => (startupOutput = await invoke('startup-output'))}>{t('showOutput')}</button>
                </div>
            {/if}
            {#if startupOutput}
                <pre class="output">{startupOutput.map((l) => `[${l.source}] ${l.content}`).join('\n') || t('noOutput')}</pre>
            {/if}
        {:else if section === 'aria2'}
            <h2>{t('section.aria2')}</h2>
            {#if app.status !== 'connected'}
                <p class="lead">{t('notConnected')}</p>
            {:else}
                <p class="lead">{t('aria2Lead')}</p>
                <div class="aria2-bar">
                    <select bind:value={category} disabled={!!filter}>
                        {#each Object.keys(globalCategories) as c}<option value={c}>{t('category.' + c)}</option>{/each}
                    </select>
                    <input type="search" bind:value={filter} placeholder={t('searchOptions')} />
                </div>
                {#if globalOptions}
                    {#each shownKeys as key (key)}
                        <OptionField
                            {key}
                            bind:value={() => edited[key] ?? toDisplay(key, globalOptions[key]), (v) => (edited[key] = v)}
                            changed={dirty.includes(key)}
                        />
                    {/each}
                {/if}
                {#if dirty.length}
                    <div class="savebar">
                        <span class="muted">{t('nChanged', { n: dirty.length })}</span>
                        <button onclick={() => (edited = {})}>{t('cancel')}</button>
                        <button class="primary" onclick={saveGlobal}>{t('saveChanges')}</button>
                    </div>
                {/if}
            {/if}
        {:else if section === 'about'}
            <h2>{t('section.about')}</h2>
            <div class="about">
                <img src="icon.svg" alt="" width="64" height="64" />
                <div>
                    <div class="appname">AriaDesk {boot.globals.version}</div>
                    <div class="muted">{t('tagline')}{boot.globals.buildCommit ? ` · ${boot.globals.buildCommit}` : ''}</div>
                    <button class="link" onclick={() => openExternal('https://aria2.github.io/')}>aria2.github.io</button>
                </div>
            </div>
            <dl class="grid">
                {#each boot.runtime as r}<dt>{r.name}</dt><dd>{r.value}</dd>{/each}
                {#if app.status === 'connected'}
                    <dt>aria2</dt><dd>{app.version}</dd>
                    <dt>{t('features')}</dt><dd>{app.features.join(', ')}</dd>
                {/if}
            </dl>
            {#if app.status === 'connected'}
                <h3>aria2</h3>
                <div class="buttons">
                    <button onclick={() => aria2Action('saveSession')}><Icon name="save" size={16} />{t('saveSession')}</button>
                    <button class="danger" onclick={() => aria2Action('shutdown', t('confirmShutdown'))}><Icon name="power" size={16} />{t('shutdown')}</button>
                </div>
            {/if}
            {#if isDesktop}
                <div class="buttons">
                    <button onclick={() => send('reload')}>{t('reload')}</button>
                    {#if devMode}<button onclick={() => send('devtools')}>DevTools</button>{/if}
                </div>
            {/if}
        {/if}
    </div>
</div>

<style>
    .settings {
        flex: 1;
        display: flex;
        min-height: 0;
    }
    nav {
        width: 210px;
        flex: none;
        padding: 16px 10px;
        border-right: 1px solid var(--border);
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    nav h1 {
        font-size: 20px;
        margin: 0 10px 14px;
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
    }
    nav a.current {
        background: var(--selected);
        color: var(--text);
        font-weight: 600;
    }
    .content {
        flex: 1;
        overflow-y: auto;
        padding: 20px 32px 40px;
        max-width: 900px;
    }
    h2 {
        margin: 0 0 6px;
        font-size: 20px;
    }
    h3 {
        margin: 28px 0 6px;
        font-size: 15px;
    }
    .lead {
        color: var(--muted);
        margin: 0 0 18px;
    }
    .form {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 12px 16px;
    }
    .form label {
        display: flex;
        flex-direction: column;
        gap: 5px;
        font-weight: 500;
    }
    .form .wide {
        grid-column: 1 / -1;
    }
    .buttons {
        display: flex;
        gap: 8px;
        margin-top: 16px;
        flex-wrap: wrap;
    }
    .note {
        margin-top: 18px;
        padding: 10px 12px;
        border-radius: 10px;
        background: var(--chip);
        font-size: 12px;
    }
    .rows {
        display: flex;
        flex-direction: column;
    }
    .row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 16px;
        padding: 12px 0;
        border-bottom: 1px solid var(--border);
    }
    .row select {
        min-width: 180px;
    }
    .output {
        max-height: 260px;
        overflow: auto;
        padding: 10px;
        background: var(--chip);
        border-radius: 8px;
        font-size: 12px;
    }
    .aria2-bar {
        display: flex;
        gap: 8px;
        position: sticky;
        top: -20px;
        padding: 8px 0;
        background: var(--bg);
        z-index: 1;
    }
    .aria2-bar input {
        flex: 1;
    }
    .savebar {
        position: sticky;
        bottom: -40px;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
        padding: 12px 0;
        background: var(--bg);
    }
    .savebar span {
        margin-right: auto;
    }
    .about {
        display: flex;
        gap: 16px;
        align-items: center;
        margin: 10px 0 20px;
    }
    .appname {
        font-size: 18px;
        font-weight: 700;
    }
    .link {
        border: none;
        background: none;
        padding: 0;
        color: var(--accent);
    }
    .grid {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 8px 18px;
    }
    dt {
        color: var(--muted);
    }
    dd {
        margin: 0;
        overflow-wrap: anywhere;
    }
    .mono {
        font-family: var(--mono);
        font-size: 12px;
    }
    @media (max-width: 860px) {
        nav {
            width: 160px;
        }
        .content {
            padding: 16px;
        }
    }
</style>
