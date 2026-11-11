// Run by `npm version`: copies the new package.json version into the Cargo workspace
// and stages the files, so the version commit and tag cover every manifest.
import fs from 'node:fs';
import { execSync } from 'node:child_process';

const { version } = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const cargo = fs.readFileSync('Cargo.toml', 'utf8');
const updated = cargo.replace(/(\[workspace\.package\][^[]*?\nversion = )"[^"]*"/, `$1"${version}"`);
if (updated === cargo && !cargo.includes(`version = "${version}"`)) {
    console.error('could not find the workspace version in Cargo.toml');
    process.exit(1);
}
fs.writeFileSync('Cargo.toml', updated);
execSync('cargo update --workspace', { stdio: 'inherit' });
execSync('git add Cargo.toml Cargo.lock', { stdio: 'inherit' });
console.log(`version set to ${version}`);
