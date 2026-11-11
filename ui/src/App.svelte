<script>
    import { onMount } from 'svelte';
    import { app, connect, toast } from './lib/store.svelte.js';
    import { settings } from './lib/settings.svelte.js';
    import { boot, on, send, fileType, listening } from './lib/native.js';
    import { t, menuLabels } from './lib/i18n.js';
    import { route, go, back } from './lib/route.svelte.js';
    import Sidebar from './components/Sidebar.svelte';
    import TaskList from './components/TaskList.svelte';
    import TaskDetail from './components/TaskDetail.svelte';
    import NewTask from './components/NewTask.svelte';
    import Settings from './components/Settings.svelte';
    import Toasts from './components/Toasts.svelte';
    import StartAria2 from './components/StartAria2.svelte';

    let dragging = $state(false);
    let devMode = $state(!!boot.globals.isDevMode);

    const listFilter = $derived(route.parts[0] === 'downloads' ? route.parts[1] || 'all' : route.lastFilter);

    // theme
    const dark = window.matchMedia('(prefers-color-scheme: dark)');
    let systemDark = $state(dark.matches);
    dark.addEventListener('change', (e) => (systemDark = e.matches));
    $effect(() => {
        const theme = settings.theme === 'system' ? (systemDark ? 'dark' : 'light') : settings.theme;
        document.documentElement.dataset.theme = theme;
        send('set-theme', settings.theme === 'system' ? '' : settings.theme);
    });

    $effect(() => {
        document.documentElement.lang = settings.language;
        send('set-menu-labels', menuLabels());
    });

    // the backend delivers pending new tasks on this message, so our listeners must be active first
    $effect(() => {
        const path = '/' + route.parts.join('/');
        listening().then(() => send('route-changed', path));
    });

    function openIncoming(incoming) {
        app.incoming = incoming;
        if (route.parts[0] !== 'new') go('/new');
    }

    function readFile(file) {
        const type = fileType(file.name);
        if (!type) {
            toast(t('invalidFileType'), 'error');
            return;
        }
        const reader = new FileReader();
        reader.onload = () => openIncoming({ type, fileName: file.name, base64Content: String(reader.result).replace(/^.*?base64,/, '') });
        reader.readAsDataURL(file);
    }

    function onDrop(e) {
        e.preventDefault();
        dragging = false;
        const file = e.dataTransfer.files?.[0];
        const text = e.dataTransfer.getData('text/uri-list') || e.dataTransfer.getData('text/plain');
        if (file) readFile(file);
        else if (text.trim()) openIncoming({ text: text.trim() });
    }

    function onKey(e) {
        const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName);
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
            e.preventDefault();
            go('/new');
        } else if (e.key === 'F12' && devMode) {
            send('devtools');
        } else if (e.key === 'Escape' && !typing && route.parts[0] === 'task') {
            back();
        }
    }

    on('navigate', (path) => go(path));
    on('new-task-file', (payload) => (payload.exception ? toast(payload.exception, 'error') : openIncoming(payload)));
    on('new-task-text', (payload) => openIncoming({ text: payload.text }));
    on('error', (message) => toast(message, 'error'));
    on('dev-mode', (value) => (devMode = value));
    on('log', (message) => console.debug(message));

    onMount(() => {
        connect();
        send('ready');
    });
</script>

<svelte:window onkeydown={onKey} />

<div
    class="shell"
    role="application"
    ondragover={(e) => {
        e.preventDefault();
        dragging = true;
    }}
    ondragleave={(e) => !e.relatedTarget && (dragging = false)}
    ondrop={onDrop}
>
    <Sidebar filter={route.parts[0] === 'downloads' ? listFilter : ''} />

    <main>
        {#if route.parts[0] === 'settings'}
            <Settings section={route.parts[1] || 'connection'} {devMode} />
        {:else}
            <div class="split" class:with-detail={route.parts[0] === 'task'}>
                <TaskList filter={listFilter} selectedGid={route.parts[0] === 'task' ? route.parts[1] : null} />
                {#if route.parts[0] === 'task'}
                    {#key route.parts[1]}
                        <TaskDetail gid={route.parts[1]} tab={route.parts[2] || 'overview'} />
                    {/key}
                {/if}
            </div>
        {/if}
    </main>

    {#if route.parts[0] === 'new'}
        <NewTask onclose={back} />
    {/if}

    {#if app.offerStart}
        <StartAria2 />
    {/if}

    {#if dragging}
        <div class="dropzone"><div>{t('dropHere')}</div></div>
    {/if}

    <Toasts />
</div>

<style>
    .shell {
        display: flex;
        height: 100vh;
        overflow: hidden;
    }
    main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        background: var(--bg);
    }
    .split {
        flex: 1;
        display: flex;
        min-height: 0;
    }
    .dropzone {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        background: color-mix(in srgb, var(--accent) 18%, transparent);
        backdrop-filter: blur(2px);
        z-index: 50;
        pointer-events: none;
    }
    .dropzone div {
        padding: 28px 40px;
        border: 2px dashed var(--accent);
        border-radius: 16px;
        background: var(--surface);
        font-size: 18px;
        font-weight: 600;
    }
</style>
