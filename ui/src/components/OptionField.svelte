<script>
    import { def, validate } from '../lib/options.js';
    import { optionLabel, optionHelp, t } from '../lib/i18n.js';

    let { key, value = $bindable(''), readonly = false, changed = false, history = [], compact = false, placeholder = undefined } = $props();

    const d = $derived(def(key));
    const valid = $derived(validate(key, value));
    const locked = $derived(readonly || d.readonly);
    const listId = $props.id();
</script>

<div class="field" class:compact title={compact ? optionHelp(key) : undefined} class:changed class:invalid={!valid && (changed || value !== '')}>
    <div class="meta">
        <label for={listId + '-input'}>{optionLabel(key)}</label>
        <code>{key}</code>
        {#if optionHelp(key) && !compact}<p>{optionHelp(key)}</p>{/if}
    </div>
    <div class="control">
        {#if d.type === 'boolean'}
            <select id={listId + '-input'} bind:value disabled={locked}>
                {#if !d.required}<option value="">{t('default')}</option>{/if}
                <option value="true">{t('yes')}</option>
                <option value="false">{t('no')}</option>
            </select>
        {:else if d.type === 'option'}
            <select id={listId + '-input'} bind:value disabled={locked}>
                {#if !d.required}<option value="">{t('default')}</option>{/if}
                {#each d.options as option}<option value={option}>{option}</option>{/each}
            </select>
        {:else if d.type === 'text'}
            <textarea id={listId + '-input'} bind:value readonly={locked} rows="3" placeholder={d.separator ? t('onePerLine') : ''}></textarea>
        {:else}
            <div class="with-suffix">
                <input
                    id={listId + '-input'}
                    type="text"
                    bind:value
                    readonly={locked}
                    placeholder={placeholder ?? d.defaultValue ?? ''}
                    list={history.length ? listId : undefined}
                    inputmode={d.type === 'integer' || d.type === 'float' ? 'decimal' : undefined}
                />
                {#if d.suffix}<span>{t('suffix.' + d.suffix)}</span>{/if}
            </div>
            {#if history.length}
                <datalist id={listId}>
                    {#each history as item}<option value={item}></option>{/each}
                </datalist>
            {/if}
        {/if}
    </div>
</div>

<style>
    .field {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(200px, 320px);
        gap: 16px;
        padding: 12px 0;
        border-bottom: 1px solid var(--border);
        align-items: start;
    }
    .compact {
        padding: 8px 0;
        align-items: center;
    }
    .meta label {
        font-weight: 500;
    }
    code {
        margin-left: 8px;
        font-size: 11px;
        color: var(--muted);
    }
    p {
        margin: 4px 0 0;
        font-size: 12px;
        color: var(--muted);
        line-height: 1.45;
        white-space: pre-line;
    }
    .control > * {
        width: 100%;
    }
    .with-suffix {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .with-suffix input {
        flex: 1;
        min-width: 0;
    }
    .with-suffix span {
        font-size: 12px;
        color: var(--muted);
    }
    .changed label::after {
        content: ' •';
        color: var(--accent);
    }
    .invalid :is(input, textarea) {
        border-color: var(--danger);
    }
    @media (max-width: 760px) {
        .field {
            grid-template-columns: 1fr;
            gap: 8px;
        }
    }
</style>
