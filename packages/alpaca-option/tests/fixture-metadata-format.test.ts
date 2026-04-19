import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { glob } from 'node:fs/promises';
import { readFile } from 'node:fs/promises';

const repoRoot = path.resolve(import.meta.dirname, '../../..');

async function fixtureFiles(): Promise<string[]> {
  const files = ['fixtures/catalog.json'];
  for await (const file of glob('fixtures/layers/**/*.json', { cwd: repoRoot })) {
    files.push(file);
  }
  return files.sort();
}

test('fixture metadata purpose strings are stored as utf8 text, not unicode escapes', async () => {
  const files = await fixtureFiles();
  const offenders: string[] = [];

  for (const relativePath of files) {
    const content = await readFile(path.join(repoRoot, relativePath), 'utf8');
    if (/"purpose"\s*:\s*"\\u/i.test(content)) {
      offenders.push(relativePath);
    }
  }

  assert.deepEqual(offenders, []);
});
