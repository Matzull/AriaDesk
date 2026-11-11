<script>
    // Dropdown drawn by the app: native <select> popups follow the toolkit theme instead of ours.
    import Icon from './Icon.svelte';

    let { value = $bindable(), options, disabled = false, id = undefined, label = undefined, ghost = false, onchange = undefined } = $props();

    let open = $state(false);
    let active = $state(0);
    let button;
    let list = $state();
    let position = $state({ left: 0, top: 0, width: 0, maxHeight: 280, up: false });

    const items = $derived(options.map((o) => (typeof o === 'object' ? o : { value: o, label: String(o) })));
    const current = $derived(items.find((o) => o.value === value));

    function show() {
        if (disabled) return;
        const rect = button.getBoundingClientRect();
        const below = window.innerHeight - rect.bottom - 8;
        const up = below < 160 && rect.top > below;
        position = {
            left: rect.left,
            top: up ? rect.top - 4 : rect.bottom + 4,
            width: rect.width,
            maxHeight: Math.min(280, (up ? rect.top : below) - 8),
            up
        };
        active = Math.max(0, items.findIndex((o) => o.value === value));
        open = true;
        queueMicrotask(() => list?.children[active]?.scrollIntoView({ block: 'nearest' }));
    }

    function choose(item) {
        open = false;
        button.focus();
        if (item.value === value) return;
        value = item.value;
        onchange?.(item.value);
    }

    function move(delta) {
        active = (active + delta + items.length) % items.length;
        list?.children[active]?.scrollIntoView({ block: 'nearest' });
    }

    function onKey(e) {
        if (!open) {
            if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
                e.preventDefault();
                show();
            }
            return;
        }
        const keys = {
            ArrowDown: () => move(1),
            ArrowUp: () => move(-1),
            Home: () => move(-active),
            End: () => move(items.length - 1 - active),
            Enter: () => choose(items[active]),
            ' ': () => choose(items[active]),
            Escape: () => (open = false),
            Tab: () => (open = false)
        };
        if (keys[e.key]) {
            if (e.key !== 'Tab') e.preventDefault();
            e.stopPropagation();
            keys[e.key]();
        } else if (e.key.length === 1) {
            // jump to the next option starting with the typed letter
            const k = e.key.toLowerCase();
            const next = items.findIndex((o, i) => i > active && o.label.toLowerCase().startsWith(k));
            const first = items.findIndex((o) => o.label.toLowerCase().startsWith(k));
            if (next >= 0 || first >= 0) move((next >= 0 ? next : first) - active);
        }
    }

    function onOutside(e) {
        if (open && !button.contains(e.target) && !list?.contains(e.target)) open = false;
    }
</script>

<svelte:window onpointerdown={onOutside} onresize={() => (open = false)} onscrollcapture={(e) => open && !list?.contains(e.target) && (open = false)} />

<button
    bind:this={button}
    {id}
    type="button"
    class="select"
    class:ghost
    class:open
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={label}
    onclick={() => (open ? (open = false) : show())}
    onkeydown={onKey}
>
    <span class="value">{current?.label ?? ''}</span>
    <Icon name="chevronDown" size={15} />
</button>

{#if open}
    <ul
        bind:this={list}
        class="menu"
        class:up={position.up}
        role="listbox"
        style:left="{position.left}px"
        style:top="{position.top}px"
        style:min-width="{position.width}px"
        style:max-height="{position.maxHeight}px"
    >
        {#each items as item, i (item.value)}
            <li
                role="option"
                aria-selected={item.value === value}
                class:active={i === active}
                class:selected={item.value === value}
                onpointerenter={() => (active = i)}
                onclick={() => choose(item)}
                onkeydown={() => {}}
            >
                <span>{item.label}</span>
                {#if item.value === value}<Icon name="check" size={14} />{/if}
            </li>
        {/each}
    </ul>
{/if}

<style>
    .select {
        display: inline-flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        min-width: 0;
        padding: 7px 10px;
        text-align: left;
        font-weight: normal;
    }
    .select .value {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .select :global(svg) {
        color: var(--muted);
        transition: transform 0.15s;
    }
    .select.open :global(svg) {
        transform: rotate(180deg);
    }
    .select.ghost {
        border-color: transparent;
        background: transparent;
        padding: 7px 4px;
    }
    .menu {
        position: fixed;
        z-index: 200;
        margin: 0;
        padding: 4px;
        list-style: none;
        overflow-y: auto;
        background: var(--surface-raised);
        border: 1px solid var(--border);
        border-radius: 10px;
        box-shadow: var(--shadow);
        animation: pop 0.1s ease-out;
    }
    .menu.up {
        transform: translateY(-100%);
        animation: none;
    }
    @keyframes pop {
        from {
            opacity: 0;
            transform: translateY(-4px);
        }
    }
    li {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        padding: 7px 10px;
        border-radius: 7px;
        cursor: pointer;
        white-space: nowrap;
    }
    li.active {
        background: var(--hover);
    }
    li.selected {
        color: var(--accent);
        font-weight: 600;
    }
</style>
