// Assembles the static frontend into dist/ (desktop) or dist-web/ (--web):
// copies app/ and only the node_modules files referenced from app/index.html (plus fonts used by their css)
// into lib/, since Tauri refuses to bundle a folder named node_modules.
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const web = process.argv.includes('--web');
const dist = path.join(root, web ? 'dist-web' : 'dist');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const indexFile = path.join(root, 'app', 'index.html');

const copy = (from, to) => {
    fs.mkdirSync(path.dirname(to), { recursive: true });
    fs.cpSync(from, to, { recursive: true });
};

const dependencies = new Set();
const html = fs.readFileSync(indexFile, 'utf8');
for (const pattern of [/<link rel="stylesheet" href="(\.\.\/node_modules\/[^"]+\.css)"\/?>/g, /<script src="(\.\.\/node_modules\/[^"]+\.js)"><\/script>/g]) {
    for (const match of html.matchAll(pattern)) {
        dependencies.add(path.join(root, 'app', match[1]));
    }
}
for (const css of [...dependencies].filter(f => f.endsWith('.css'))) {
    const content = fs.readFileSync(css, 'utf8');
    for (const match of content.matchAll(/url\(['"]?(\.\.\/fonts\/[a-zA-Z0-9\-]+\.[a-zA-Z0-9]+)(\?[^'")]*)?(#[^'")]*)?['"]?\)/g)) {
        dependencies.add(path.join(path.dirname(css), match[1]));
    }
}

fs.rmSync(dist, { recursive: true, force: true });
copy(path.join(root, 'app'), path.join(dist, 'app'));
for (const file of dependencies) {
    if (!fs.existsSync(file)) {
        console.error(`missing dependency ${path.relative(root, file)}, run npm install`);
        process.exit(1);
    }
    copy(file, path.join(dist, 'lib', path.relative(path.join(root, 'node_modules'), file)));
}

const distIndex = path.join(dist, 'app', 'index.html');
fs.writeFileSync(distIndex, fs.readFileSync(distIndex, 'utf8').replaceAll('"../node_modules/', '"../lib/'));

if (web) {
    let commit = '';
    try {
        commit = execSync('git rev-parse --short HEAD', { cwd: root }).toString().trim();
    } catch (ex) {
        // Do Nothing
    }
    const boot = {
        platform: 'web',
        os: 'web',
        sep: '/',
        globals: { version: pkg.version, buildCommit: commit, isDevMode: false, useCustomAppTitle: false },
        nativeConfig: {},
        runtime: [{ name: 'Web', value: 'Browser' }]
    };
    fs.writeFileSync(distIndex, fs.readFileSync(distIndex, 'utf8')
        .replace('<!-- ARIADESK_WEB_BOOT -->', `<script>window.__ARIADESK_BOOT__ = ${JSON.stringify(boot)};</script>`));
    fs.writeFileSync(path.join(dist, 'index.html'), '<!DOCTYPE html><meta http-equiv="refresh" content="0; url=app/index.html">\n');
}

console.log(`frontend written to ${path.relative(root, dist)} (${dependencies.size} vendor files)`);
