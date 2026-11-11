<script>
    import { app } from '../lib/store.svelte.js';
</script>

<div class="toasts" aria-live="polite">
    {#each app.toasts as toast (toast.id)}
        <div class="toast {toast.kind}">{toast.message}</div>
    {/each}
</div>

<style>
    .toasts {
        position: fixed;
        right: 16px;
        bottom: 16px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        z-index: 100;
        max-width: 380px;
    }
    .toast {
        padding: 10px 14px;
        border-radius: 10px;
        background: var(--surface-raised);
        border: 1px solid var(--border);
        border-left: 4px solid var(--accent);
        box-shadow: var(--shadow);
        animation: slide 0.18s ease-out;
    }
    .toast.error {
        border-left-color: var(--danger);
    }
    .toast.ok {
        border-left-color: var(--ok);
    }
    @keyframes slide {
        from {
            transform: translateY(8px);
            opacity: 0;
        }
    }
</style>
