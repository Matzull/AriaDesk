<script>
    import Icon from './Icon.svelte';
    import { app, startAria2 } from '../lib/store.svelte.js';
    import { rpcUrl } from '../lib/settings.svelte.js';
    import { boot, setNativeConfig } from '../lib/native.js';
    import { t } from '../lib/i18n.js';

    let always = $state(!!boot.nativeConfig.autoStartAria2);
    let busy = $state(false);
    let error = $state('');

    const close = () => (app.offerStart = false);

    async function start() {
        if (always !== !!boot.nativeConfig.autoStartAria2) setNativeConfig('autoStartAria2', always);
        busy = true;
        error = (await startAria2()) || '';
        busy = false;
        if (!error) close();
    }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !busy && close()} />

<div class="backdrop" role="presentation">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="start-title" aria-describedby="start-text">
        <div class="icon"><Icon name="power" size={22} /></div>
        <h2 id="start-title">{t('aria2NotRunning')}</h2>
        <p id="start-text">{t('aria2NotRunningText', { url: rpcUrl() })}</p>
        <p class="muted small">{t('aria2BackgroundNote')}</p>

        <label class="always">
            <input type="checkbox" bind:checked={always} />
            {t('aria2Always')}
        </label>

        {#if error}<div class="error">{error}</div>{/if}

        <div class="buttons">
            <button onclick={close} disabled={busy}>{t('notNow')}</button>
            <button class="primary" onclick={start} disabled={busy}>
                {busy ? t('starting') : error ? t('retry') : t('startAria2')}
            </button>
        </div>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgb(0 0 0 / 0.35);
        display: grid;
        place-items: center;
        z-index: 40;
    }
    .dialog {
        width: min(460px, calc(100vw - 32px));
        padding: 24px;
        background: var(--surface-raised);
        border: 1px solid var(--border);
        border-radius: 16px;
        box-shadow: var(--shadow);
        display: flex;
        flex-direction: column;
        gap: 10px;
    }
    .icon {
        width: 42px;
        height: 42px;
        border-radius: 12px;
        display: grid;
        place-items: center;
        color: var(--accent);
        background: color-mix(in srgb, var(--accent) 14%, transparent);
    }
    h2 {
        margin: 4px 0 0;
        font-size: 18px;
    }
    p {
        margin: 0;
        line-height: 1.5;
    }
    .small {
        font-size: 12px;
    }
    .always {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-top: 6px;
    }
    .error {
        padding: 10px 12px;
        border-radius: 10px;
        color: var(--danger);
        background: color-mix(in srgb, var(--danger) 10%, transparent);
        overflow-wrap: anywhere;
    }
    .buttons {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        margin-top: 8px;
    }
</style>
