// Bumps the patch version everywhere it lives, then commits all changes.
// Usage: npm run save -- "optional message"
import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';

const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
const [maj, min, patch] = pkg.version.split('.').map(Number);
const next = `${maj}.${min}.${patch + 1}`;

const edit = (file, from, to) => {
  const src = readFileSync(file, 'utf8');
  const out = src.replace(from, to);
  if (out === src) throw new Error(`version not found in ${file}`);
  writeFileSync(file, out);
};

edit('package.json', /"version": "[^"]+"/, `"version": "${next}"`);
edit('src-tauri/tauri.conf.json', /"version": "[^"]+"/, `"version": "${next}"`);
edit('src-tauri/Cargo.toml', /^version = "[^"]+"/m, `version = "${next}"`);
const crate = readFileSync('src-tauri/Cargo.toml', 'utf8').match(/^name = "([^"]+)"/m)[1];
edit('src-tauri/Cargo.lock', new RegExp(`(name = "${crate}"\\r?\\nversion = )"[^"]+"`), `$1"${next}"`);

const msg = process.argv[2] ? `v${next}: ${process.argv[2]}` : `v${next}`;
execSync('git add -A', { stdio: 'inherit' });
execSync(`git commit -m ${JSON.stringify(msg)}`, { stdio: 'inherit' });
console.log(`Committed ${msg}`);
