import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { PocketDropClient, endpoint } from './client.mjs';
import { executePocketDrop } from './tools.mjs';
test('reject public, insecure and URL credentials', () => {
  for (const url of ['http://192.168.1.2', 'https://example.com', 'https://8.8.8.8', 'https://x@y/', 'https://192.168.1.2/path']) assert.throws(() => endpoint(url));
  assert.equal(endpoint('https://192.168.1.2:54321'), 'https://192.168.1.2:54321');
});
test('require explicit local user request before accessing Room', async () => {
  let accessed = false;
  const factory = async () => { accessed = true; return { state: async () => ({ text: 'ok', revision: 1 }) }; };
  const call = { tool: 'pocketdrop_read_text', args: {} };
  for (const context of [{ source: 'idle', userText: 'PocketDrop' }, { source: 'user', userText: 'hello' }, { source: 'user', userText: 'PocketDrop', deviceId: 'remote' }]) await assert.rejects(executePocketDrop(call, context, factory));
  assert.equal(accessed, false);
  assert.deepEqual(await executePocketDrop(call, { source: 'user', userText: '讀取 PocketDrop' }, factory), { content: 'ok', revision: 1 });
  await assert.rejects(executePocketDrop({ tool: 'pocketdrop_write_text', args: { content: 'x' } }, { source: 'user', userText: '讀取 PocketDrop' }, factory));
});
test('real PocketDrop: pair, read, write, revoke, wrong TLS pin', { skip: !process.env.PD_INTEROP_EXE }, async () => {
  const root = resolve('.test-runs', 'daily-api-' + randomUUID()); await mkdir(root, { recursive: true });
  const child = spawn(process.env.PD_INTEROP_EXE, ['--phone-smoke', root], { windowsHide: true, stdio: 'ignore' });
  try {
    let invite;
    for (let i = 0; i < 100; i++) { try { invite = JSON.parse(await readFile(join(root, 'bootstrap.json'), 'utf8')); break; } catch { await new Promise(r => setTimeout(r, 100)); } }
    assert.ok(invite);
    const client = await PocketDropClient.pair({ ...invite, app: 'PocketDrop', protocol_version: 1 });
    await client.writeText('Daily-Agent → PocketDrop 測試');
    assert.equal((await client.state()).text, 'Daily-Agent → PocketDrop 測試');
    assert.deepEqual(await executePocketDrop({ tool: 'pocketdrop_list_files', args: {} }, { source: 'user', userText: 'PocketDrop 檔案' }, async () => client), { files: [] });
    await assert.rejects(new PocketDropClient({ ...client.profile, certificate_sha256: '0'.repeat(64) }).request('/v1/state'), /憑證不符/);
    await assert.rejects(client.writeText('中'.repeat(11000)));
    const db = new DatabaseSync(join(root, 'phone-trial.sqlite')); db.exec('UPDATE members SET revoked=1'); db.close();
    await assert.rejects(client.state(), /401/);
  } finally { child.kill(); }
});
