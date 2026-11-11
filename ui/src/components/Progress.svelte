<script>
    let { value, status = 'active', height = 4 } = $props();
</script>

<div class="track" style:height="{height}px">
    <div class="bar {status}" style:width="{value}%"></div>
</div>

<style>
    .track {
        width: 100%;
        background: var(--track);
        border-radius: 99px;
        overflow: hidden;
    }
    .bar {
        height: 100%;
        background: var(--accent);
        border-radius: 99px;
        transition: width 0.4s ease;
    }
    .bar.paused,
    .bar.waiting {
        background: var(--muted);
    }
    .bar.complete {
        background: var(--ok);
    }
    .bar.error,
    .bar.removed {
        background: var(--danger);
    }
</style>
