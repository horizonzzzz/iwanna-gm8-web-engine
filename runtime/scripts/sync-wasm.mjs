import { copyFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const runtimeDir = path.resolve(scriptDir, '..');
const repoDir = path.resolve(runtimeDir, '..');
const sourcePath = path.join(repoDir, 'target', 'wasm32-unknown-unknown', 'release', 'iwm_runtime_web.wasm');
const targetDir = path.join(runtimeDir, 'public', 'wasm');
const targetPath = path.join(targetDir, 'iwm_runtime_web.wasm');

await mkdir(targetDir, { recursive: true });
await copyFile(sourcePath, targetPath);

process.stdout.write(`synced ${sourcePath} -> ${targetPath}\n`);
