import { readFile } from 'node:fs/promises';
import { PNG } from 'pngjs';
import jsQR from 'jsqr';
import { PocketDropClient } from './client.mjs';
import { saveProfile } from './profile.mjs';
try {
  const file = process.argv[2];
  if (!file) throw Error('用法：node pair.mjs 邀請QR截圖.png');
  const bytes = await readFile(file);
  if (bytes.length > 10 * 1024 * 1024) throw Error('圖片上限 10 MB');
  // Check dimensions before decompressing an untrusted PNG.
  if (bytes.length < 24 || bytes.readUInt32BE(16) * bytes.readUInt32BE(20) > 16000000) throw Error('圖片尺寸過大');
  const png = PNG.sync.read(bytes);
  const qr = jsQR(new Uint8ClampedArray(png.data), png.width, png.height);
  if (!qr) throw Error('找不到 QR Code，請截取完整邀請 QR 圖片');
  const client = await PocketDropClient.pair(JSON.parse(qr.data));
  await saveProfile(client.profile);
  console.log('Daily-Agent 已加入 PocketDrop。配對資料已用 Windows DPAPI 保存。請刪除邀請截圖。');
} catch (error) { console.error(error.message); process.exitCode = 1; }
