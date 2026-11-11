import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { execSync } from 'node:child_process';
import pkg from './package.json' with { type: 'json' };

let commit = '';
try {
    commit = execSync('git rev-parse --short HEAD').toString().trim();
} catch {
    // not a git checkout
}

export default defineConfig({
    root: 'ui',
    base: './',
    plugins: [svelte()],
    define: {
        __APP_VERSION__: JSON.stringify(pkg.version),
        __BUILD_COMMIT__: JSON.stringify(commit)
    },
    build: { outDir: '../dist', emptyOutDir: true, target: 'es2022' },
    server: { port: 1420, strictPort: true },
    clearScreen: false
});
