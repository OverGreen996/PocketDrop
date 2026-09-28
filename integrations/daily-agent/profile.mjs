import { spawn } from 'node:child_process';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, dirname } from 'node:path';
export const profilePath = join(process.env.LOCALAPPDATA ?? '.', 'PocketDrop', 'daily-agent.dpapi');
async function protect(input, decrypt = false) {
  if (process.platform !== 'win32') throw Error('此介接模組使用 Windows DPAPI 儲存配對');
  const action = decrypt ? 'Unprotect' : 'Protect';
  const script = `Add-Type -AssemblyName System.Security; $b=[Convert]::FromBase64String([Console]::In.ReadToEnd()); $r=[Security.Cryptography.ProtectedData]::${action}($b,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser); [Console]::Write([Convert]::ToBase64String($r))`;
  return new Promise((resolve, reject) => {
    const p = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], { windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'] });
    let result = ''; p.stdout.on('data', b => result += b); p.on('error', reject);
    p.on('close', code => code === 0 ? resolve(Buffer.from(result, 'base64')) : reject(Error('DPAPI 配對資料讀寫失敗')));
    p.stdin.on('error', reject); p.stdin.end(Buffer.from(input).toString('base64'));
  });
}
export async function saveProfile(profile) {
  const encrypted = await protect(JSON.stringify(profile));
  await mkdir(dirname(profilePath), { recursive: true });
  await writeFile(profilePath, encrypted);
}
export async function loadProfile() { return JSON.parse((await protect(await readFile(profilePath), true)).toString('utf8')); }
