<script>
    let { history, height = 46, points = 90 } = $props();

    const max = $derived(Math.max(1024, ...history.map((p) => Math.max(p.d, p.u))));

    function line(key) {
        const offset = points - history.length;
        return history.map((p, i) => `${((offset + i) / (points - 1)) * 100},${height - (p[key] / max) * (height - 2) - 1}`).join(' ');
    }
    const area = (key) => (history.length > 1 ? `${line(key)} 100,${height} ${((points - history.length) / (points - 1)) * 100},${height}` : '');
</script>

<svg viewBox="0 0 100 {height}" preserveAspectRatio="none" style:height="{height}px">
    <polygon points={area('d')} class="fill-d" />
    <polyline points={line('u')} class="u" />
    <polyline points={line('d')} class="d" />
</svg>

<style>
    svg {
        width: 100%;
        display: block;
    }
    polyline {
        fill: none;
        stroke-width: 1.5;
        vector-effect: non-scaling-stroke;
    }
    .d {
        stroke: var(--down);
    }
    .u {
        stroke: var(--up);
    }
    .fill-d {
        fill: color-mix(in srgb, var(--down) 15%, transparent);
    }
</style>
