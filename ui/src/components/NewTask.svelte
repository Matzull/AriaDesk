<script>
    import { onMount } from 'svelte';
    import Icon from './Icon.svelte';
    import OptionField from './OptionField.svelte';
    import { app, rpc, toast, refresh } from '../lib/store.svelte.js';
    import { settings, rememberDir } from '../lib/settings.svelte.js';
    import { taskOptions, taskOptionAllowed, toSubmit, validate } from '../lib/options.js';
    import { parseTorrent, fileType } from '../lib/native.js';
    import { bytes } from '../lib/format.js';
    import { t } from '../lib/i18n.js';

    let { onclose } = $props();

    let links = $state('');
    let file = $state(null); // { type, fileName, base64Content }
    let torrent = $state(null);
    let selected = $state([]);
    let values = $state(Object.fromEntries(taskOptions.map((o) => [o.key, ''])));
    let advanced = $state(false);
    let busy = $state(false);
    let globalDir = $state('');
    let picker;
    let textarea;

    const quick = ['dir', 'out', 'split', 'max-connection-per-server', 'max-download-limit'];
    const advancedOptions = taskOptions.filter((o) => !quick.includes(o.key) && taskOptionAllowed(o, 'new', 'show'));
    const lines = $derived(links.split(/\r?\n/).map((s) => s.trim()).filter(Boolean));

    function accept(incoming) {
        if (!incoming) return;
        if (incoming.base64Content) {
            setFile(incoming);
        } else if (incoming.text) {
            links = links.trim() ? links.trim() + '\n' + incoming.text : incoming.text;
        }
        app.incoming = null;
    }

    $effect(() => accept(app.incoming));

    function setFile(f) {
        file = f;
        torrent = f.type === 'torrent' ? parseTorrent(f.base64Content) : null;
        selected = torrent ? torrent.files.map(() => true) : [];
    }

    function pick(e) {
        const f = e.target.files[0];
        e.target.value = '';
        if (!f) return;
        const type = fileType(f.name);
        if (!type) return toast(t('invalidFileType'), 'error');
        const reader = new FileReader();
        reader.onload = () => setFile({ type, fileName: f.name, base64Content: String(reader.result).replace(/^.*?base64,/, '') });
        reader.readAsDataURL(f);
    }

    onMount(() => textarea?.focus());

    $effect(() => {
        if (app.status === 'connected' && !globalDir) {
            rpc('getGlobalOption').then((o) => (globalDir = o.dir || ''), () => {});
        }
    });

    function collectOptions(paused) {
        const options = {};
        for (const [key, value] of Object.entries(values)) {
            if (value === '' || value == null) continue;
            if (!validate(key, value)) throw new Error(t('invalidValue', { name: key }));
            options[key] = toSubmit(key, value);
        }
        if (paused) options.pause = 'true';
        if (torrent && selected.some((s) => !s)) {
            const indexes = selected.map((on, i) => (on ? i + 1 : 0)).filter(Boolean);
            if (!indexes.length) throw new Error(t('selectOneFile'));
            options['select-file'] = indexes.join(',');
        }
        return options;
    }

    async function submit(paused) {
        let options;
        try {
            options = collectOptions(paused);
        } catch (e) {
            return toast(e.message, 'error');
        }
        busy = true;
        try {
            if (file) {
                if (file.type === 'torrent') await rpc('addTorrent', file.base64Content, [], options);
                else await rpc('addMetalink', file.base64Content, options);
            }
            for (const line of lines) {
                // a line may hold several mirrors of the same file separated by spaces
                await rpc('addUri', line.split(/\s+/), options);
            }
            rememberDir(values.dir);
            toast(t('added'), 'ok');
            refresh();
            onclose();
        } catch (e) {
            toast(e.message, 'error');
        } finally {
            busy = false;
        }
    }

    function onKey(e) {
        if (e.key === 'Escape') onclose();
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && canSubmit) submit(false);
    }

    const canSubmit = $derived(!busy && (lines.length > 0 || !!file) && app.status === 'connected');
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="new-title">
        <header>
            <h2 id="new-title">{t('addDownload')}</h2>
            <button class="icon ghost" onclick={onclose} title={t('close')}><Icon name="x" /></button>
        </header>

        <div class="body">
            <label class="block-label" for="links">{t('links')}</label>
            <textarea id="links" bind:this={textarea} bind:value={links} rows="4" placeholder={t('linksPlaceholder')} spellcheck="false"></textarea>

            <div class="file-row">
                {#if file}
                    <div class="chip">
                        <Icon name="file" size={16} />
                        <span>{file.fileName}</span>
                        <button class="icon ghost" onclick={() => ((file = null), (torrent = null))} title={t('remove')}><Icon name="x" size={14} /></button>
                    </div>
                {:else}
                    <button onclick={() => picker.click()}><Icon name="file" size={16} />{t('chooseTorrent')}</button>
                    <span class="muted">{t('orDrop')}</span>
                {/if}
                <input bind:this={picker} type="file" accept=".torrent,.meta4,.metalink" hidden onchange={pick} />
            </div>

            {#if torrent}
                <div class="torrent">
                    <div class="tname">{torrent.name} <span class="muted">· {bytes(torrent.length)}</span></div>
                    {#if torrent.files.length > 1}
                        <div class="tfiles">
                            <label class="tfile all">
                                <input
                                    type="checkbox"
                                    checked={selected.every(Boolean)}
                                    onchange={(e) => (selected = selected.map(() => e.target.checked))}
                                />
                                <span>{t('allFiles', { n: torrent.files.length })}</span>
                            </label>
                            {#each torrent.files as f, i}
                                <label class="tfile">
                                    <input type="checkbox" bind:checked={selected[i]} />
                                    <span class="path">{f.path}</span>
                                    <span class="muted">{bytes(f.length)}</span>
                                </label>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}

            <div class="quick">
                <OptionField compact key="dir" bind:value={values.dir} history={[...settings.dirHistory, globalDir].filter(Boolean)} placeholder={globalDir} />
                {#if lines.length <= 1 && !file}
                    <OptionField compact key="out" bind:value={values.out} />
                {/if}
                <OptionField compact key="split" bind:value={values.split} />
                <OptionField compact key="max-connection-per-server" bind:value={values['max-connection-per-server']} />
                <OptionField compact key="max-download-limit" bind:value={values['max-download-limit']} />
            </div>

            <button class="disclosure" onclick={() => (advanced = !advanced)}>
                <Icon name={advanced ? 'chevronUp' : 'chevronDown'} size={16} />{t('moreOptions')}
            </button>
            {#if advanced}
                {#each advancedOptions as o (o.key)}
                    <OptionField compact key={o.key} bind:value={values[o.key]} />
                {/each}
            {/if}
        </div>

        <footer>
            <span class="muted hint">Ctrl+Enter</span>
            <button disabled={!canSubmit} onclick={() => submit(true)}><Icon name="pause" size={16} />{t('addPaused')}</button>
            <button class="primary" disabled={!canSubmit} onclick={() => submit(false)}><Icon name="down" size={16} />{t('download')}</button>
        </footer>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgb(0 0 0 / 0.35);
        display: grid;
        place-items: center;
        z-index: 30;
        animation: fade 0.12s ease-out;
    }
    @keyframes fade {
        from {
            opacity: 0;
        }
    }
    .dialog {
        width: min(720px, calc(100vw - 32px));
        max-height: calc(100vh - 48px);
        display: flex;
        flex-direction: column;
        background: var(--surface-raised);
        border-radius: 16px;
        box-shadow: var(--shadow);
        border: 1px solid var(--border);
    }
    header,
    footer {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 14px 18px;
    }
    header h2 {
        flex: 1;
        margin: 0;
        font-size: 17px;
    }
    footer {
        justify-content: flex-end;
        border-top: 1px solid var(--border);
    }
    .hint {
        margin-right: auto;
        font-size: 12px;
    }
    .body {
        overflow-y: auto;
        padding: 0 18px 12px;
    }
    .block-label {
        display: block;
        font-weight: 500;
        margin-bottom: 6px;
    }
    textarea#links {
        width: 100%;
        font-family: var(--mono);
        font-size: 12px;
        resize: vertical;
    }
    .file-row {
        display: flex;
        align-items: center;
        gap: 10px;
        margin: 10px 0;
    }
    .chip {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 4px 4px 4px 10px;
        border-radius: 8px;
        background: var(--chip);
    }
    .torrent {
        border: 1px solid var(--border);
        border-radius: 10px;
        padding: 10px 12px;
        margin-bottom: 8px;
    }
    .tname {
        font-weight: 600;
        overflow-wrap: anywhere;
    }
    .tfiles {
        max-height: 200px;
        overflow-y: auto;
        margin-top: 8px;
    }
    .tfile {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 3px 0;
        font-size: 12px;
    }
    .tfile.all {
        font-weight: 500;
        border-bottom: 1px solid var(--border);
        padding-bottom: 6px;
    }
    .tfile .path {
        flex: 1;
        overflow-wrap: anywhere;
    }
    .disclosure {
        border: none;
        background: none;
        padding: 12px 0 4px;
        color: var(--accent);
    }
</style>
