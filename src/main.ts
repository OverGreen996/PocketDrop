import './style.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import QRCode from 'qrcode';
import { BrowserQRCodeReader, IScannerControls } from '@zxing/browser';

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
<main class="widget">
 <header><div class="brand" id="drag-handle"><span class="brand-icon">◇</span><div><h1>PocketDrop<span class="version">1.0</span></h1><p id="room-status">正在準備你的共享空間…</p></div></div><div class="window-actions"><button id="pin" aria-label="永遠置頂" aria-pressed="false">◇</button><button id="minimize" aria-label="最小化">−</button><button id="close" aria-label="關閉">×</button></div></header>
 <div class="intro"><span class="eyebrow">YOUR LITTLE SHARED SPACE</span><p>貼進去，丟進去，拿出來。</p></div>
 <section class="card text-card"><div class="section-head"><h2>Shared Text</h2><span class="tag" id="text-state">等待連線</span></div><textarea id="text" spellcheck="false" maxlength="32768" aria-label="共享文字" placeholder="貼上文字，按分享，Room 裡的裝置就會看到。"></textarea><div class="card-foot"><button id="latest">載入最新</button><button id="clear-text">清空</button><button id="copy">複製</button><button id="share-text" class="primary">分享文字 ↗</button></div></section>
 <section class="card files-card" id="files"><div class="section-head"><h2>Shared Files</h2><span class="tag" id="file-count">0 個檔案</span></div><div class="drop-zone"><span class="drop-icon">↓</span><strong id="drop-title">把檔案放到這裡</strong><span id="drop-subtitle">其他裝置按下載後才會取得檔案</span></div><ul id="file-list" aria-label="Room 檔案"></ul><p class="hint">下載存入「下載／PocketDrop」</p><button id="downloads">開啟下載資料夾</button><p id="transfer-status" class="hint"></p><button id="cancel-download" hidden>取消下載</button></section>
 <section class="card"><div class="section-head"><h2>自己的裝置</h2><button class="primary" id="pair">邀請裝置 ↗</button></div><div class="control-row"><button id="join">加入既有 Room</button><button id="leave" hidden>返回自己的 Room</button></div><ul id="devices"></ul><p class="hint">手機或另一台電腦都可用驗證碼或 QR 加入。<br>兩端使用同一個 Wi-Fi，電腦需保持開啟。</p></section>
 <details class="settings"><summary>連線與外觀設定</summary><div class="control-row"><label for="interface">本機網路</label><select id="interface"></select><button id="connect">啟動</button></div><p class="hint" id="network-status">若 Windows 詢問防火牆，請允許信任的私人網路，讓手機可以連線。</p><div class="control-row"><label for="material">背景材質</label><select id="material"><option value="acrylic">原生 Acrylic</option><option value="off">關閉模糊（對照）</option></select><button id="contrast">◐</button></div><p class="hint" id="material-status"></p><button id="backdrop">開啟外部背景測試板 ↗</button></details>
 <footer><span>1.0.1 · 建立 Room 的電腦需開啟</span><span>LAN ONLY</span></footer><div id="toast" role="status" aria-live="polite"></div>
 <dialog id="pair-dialog"><div class="section-head"><h2>讓裝置加入 Room</h2><button id="close-pair">×</button></div><p>在另一台裝置輸入驗證碼，或掃描 QR Code</p><p id="invite-host" class="hint"></p><strong id="pair-code" class="pair-code"></strong><button id="copy-code">複製驗證碼</button><canvas id="qr"></canvas><p id="qr-expiry"></p><p class="hint">邀請只能使用一次。請勿轉傳 QR Code。</p><button id="refresh-qr">產生新邀請</button></dialog>
<dialog id="join-dialog"><div class="section-head"><h2>加入 Room</h2><button id="close-join">×</button></div><p>在建立 Room 的電腦按「邀請裝置」</p><label for="nearby">附近的電腦</label><select id="nearby"></select><button id="scan-rooms">重新尋找</button><input id="verification" inputmode="numeric" maxlength="8" autocomplete="off" placeholder="8 位驗證碼" aria-label="驗證碼"><button id="join-code" class="primary">使用驗證碼加入</button><hr><button id="scan-camera">掃描 QR Code</button><label class="qr-file">或選取 QR 圖片<input id="qr-image" type="file" accept="image/*"></label><video id="camera" muted playsinline hidden></video><p class="hint" id="join-status">請使用同一個 Wi-Fi／區域網路。</p></dialog>
</main>`;
const $ = <T extends HTMLElement>(s: string) => document.querySelector<T>(s)!;
let toastTimer = 0;
function toast(message: string) { $('#toast').textContent = message; $('#toast').classList.add('show'); clearTimeout(toastTimer); toastTimer = window.setTimeout(() => $('#toast').classList.remove('show'), 5000); }
const size = (n: number) => { const i = n ? Math.min(4, Math.floor(Math.log(n) / Math.log(1024))) : 0; return `${(n / 1024 ** i).toFixed(i ? 1 : 0)} ${['B','KB','MB','GB','TB'][i]}`; };
interface Snapshot { text: string; revision: number; room_name: string; device_id: string; endpoint: string; files: { file_id: string; name: string; size: number; available: boolean; origin: string }[]; devices: {device_id: string; name: string}[] }
let snapshot: Snapshot | undefined;
let joined = false; let online = true; let refreshing = false;
let dirty = false;
let expiryTimer = 0;
let lastMembers = 0;
function render(state: Snapshot) {
 const previous = snapshot; snapshot = state;
 $('#room-status').textContent = `● ${state.room_name} · ${state.devices.length} 台已配對`;
 if (!dirty) $<HTMLTextAreaElement>('#text').value = state.text;
 $('#text-state').textContent = dirty ? (previous && previous.revision !== state.revision ? '有新文字 · 草稿已保留' : '尚未分享的草稿') : `已同步 · ${state.revision}`;
 $('#file-count').textContent = `${state.files.length} 個檔案`;
 $('#files').classList.toggle('has-files', state.files.length > 0);
 $('#file-list').replaceChildren(...state.files.map(file => {
  const li = document.createElement('li'); const name = document.createElement('span'); name.textContent = file.name; name.title = `${file.name} · ${file.origin}`;
  const meta = document.createElement('small'); meta.textContent = file.available && online ? size(file.size) : '來源離線或檔案已變更';
  const remove = document.createElement('button'); remove.textContent = '移除'; remove.title = '僅移除共享紀錄，保留原檔'; remove.onclick = () => void invoke('phone_remove', {fileId: file.file_id}).catch(e => toast(String(e)));
  li.append(name, meta);
  { const download = document.createElement('button'); download.textContent = '下載'; download.disabled = !file.available || !online; download.onclick = () => void downloadFile(file.file_id); li.append(download); }
  if (!joined) li.append(remove);
  return li;
 }));
 $('#devices').replaceChildren(...state.devices.map(device => {
  const li = document.createElement('li'); const name = document.createElement('span'); name.textContent = device.name;
  const revoke = document.createElement('button'); revoke.textContent = '移除裝置'; revoke.onclick = () => void invoke('phone_revoke', {deviceId: device.device_id}).then(() => toast('已撤銷此裝置的存取權')).catch(e => toast(String(e)));
  li.append(name); if (!joined) li.append(revoke); return li;
 }));
 if (state.devices.length > lastMembers && $<HTMLDialogElement>('#pair-dialog').open) { $<HTMLDialogElement>('#pair-dialog').close(); clearInterval(expiryTimer); toast('✓ 裝置已加入 Room'); }
 lastMembers = state.devices.length;
}
async function refresh() { if (refreshing) return; refreshing = true; try { joined = (await invoke<string>('room_mode')) === 'joined'; $('#leave').hidden = !joined; $('#pair').hidden = joined; $('#join').hidden = joined; render(await invoke<Snapshot>('phone_snapshot')); if (joined && !online) $('#room-status').textContent = '○ 電腦離線 · 正在重新尋找，配對已保存'; } catch (e) { $('#room-status').textContent = String(e); } finally { refreshing = false; } }
async function share(content: string) { try { await invoke('phone_text', {content}); dirty = false; await refresh(); toast('✓ 已分享文字'); } catch (e) { toast(String(e)); } }
$('#text').oninput = () => { dirty = true; $('#text-state').textContent = '尚未分享的草稿'; };
$('#share-text').onclick = () => void share($<HTMLTextAreaElement>('#text').value);
$('#clear-text').onclick = () => void share('');
$('#latest').onclick = () => { dirty = false; void refresh(); };
$('#copy').onclick = () => void navigator.clipboard.writeText($<HTMLTextAreaElement>('#text').value).then(() => toast('✓ 已複製')).catch(() => toast('請選取文字後按 Ctrl+C'));
$('#contrast').onclick = () => document.body.classList.toggle('light-ink');
async function invite() {
 try {
  const result = await invoke<{qr: string; expires_seconds: number; code: string}>('phone_invite');
  $('#pair-code').textContent = result.code; $('#invite-host').textContent = `這台電腦：PocketDrop · ${snapshot?.device_id.slice(0,8) ?? ''}`;
  await QRCode.toCanvas($<HTMLCanvasElement>('#qr'), result.qr, {width: 320, margin: 3, errorCorrectionLevel: 'M'});
  const dialog = $<HTMLDialogElement>('#pair-dialog'); if (!dialog.open) dialog.showModal();
  let remaining = result.expires_seconds; clearInterval(expiryTimer); $('#qr-expiry').textContent = `邀請有效 ${remaining} 秒`;
  expiryTimer = window.setInterval(() => { remaining--; $('#qr-expiry').textContent = remaining > 0 ? `邀請有效 ${remaining} 秒` : '邀請已過期，請產生新邀請'; if (remaining <= 0) { clearInterval(expiryTimer); $<HTMLCanvasElement>('#qr').getContext('2d')?.clearRect(0, 0, 400, 400); } }, 1000);
 } catch (e) { toast(String(e)); }
}
$('#pair').onclick = () => void invite(); $('#refresh-qr').onclick = () => void invite(); $('#close-pair').onclick = () => { $<HTMLDialogElement>('#pair-dialog').close(); clearInterval(expiryTimer); };
async function init() {
 if (!isTauri()) { $('#room-status').textContent = '請啟動 Windows 執行檔'; return; }
 const win = getCurrentWindow(); await invoke('initialize_window');
 const syncShape=()=>void invoke('sync_window_shape',{viewportWidth:window.innerWidth}).catch(e=>toast(String(e)));
 window.addEventListener('resize',syncShape); syncShape();
 $('#drag-handle').onpointerdown = e => { if (e.button === 0) void win.startDragging(); };
 $('#close').onclick = () => void win.close(); $('#minimize').onclick = () => void win.minimize();
 let pinned = false; $('#pin').onclick = async () => { try { await win.setAlwaysOnTop(!pinned); pinned = !pinned; $('#pin').setAttribute('aria-pressed', String(pinned)); } catch (e) { toast(String(e)); } };
 $('#backdrop').hidden = true; $('#backdrop').onclick = () => void invoke('open_backdrop').catch(e => toast(String(e)));
 const material = async () => { try { $('#material-status').textContent = await invoke<string>('set_material', {mode: $<HTMLSelectElement>('#material').value}); } catch (e) { $('#material-status').textContent = `原生材質失敗：${String(e)}`; } };
 $<HTMLSelectElement>('#material').onchange = () => void material(); await material();
 await listen('phone-changed', () => void refresh());
 await listen<boolean>('room-online', ({payload}) => { online = payload; void refresh(); });
 await listen<{done:number;total:number;speed:number;seconds:number}>('download-progress', ({payload:p}) => { $('#transfer-status').textContent = `${size(p.done)} / ${size(p.total)} · ${size(p.speed)}/s · 約 ${Math.ceil(p.seconds)} 秒`; });
 await listen<boolean>('drop-hover', ({payload}) => { document.body.classList.toggle('drag-over', payload); $('#drop-title').textContent = payload ? '放開以加入' : '把檔案放到這裡'; });
 await listen<{accepted: number; error?: string}>('phone-drop', ({payload}) => { toast(payload.error ? `${payload.accepted} 個已加入；${payload.error}` : `✓ ${payload.accepted} 個檔案已加入共享區`); void refresh(); });
 const interfaces = await invoke<{name: string; address: string}[]>('interfaces');
 interfaces.sort((a,b) => Number(/virtual|vethernet|vmware|vpn|wsl/i.test(a.name)) - Number(/virtual|vethernet|vmware|vpn|wsl/i.test(b.name)) || Number(/wi-fi|wireless|wlan/i.test(b.name)) - Number(/wi-fi|wireless|wlan/i.test(a.name)));
 $('#interface').replaceChildren(...interfaces.map(i => { const o = document.createElement('option'); o.value = i.address; o.textContent = i.name; return o; }));
 async function connect() { const button = $<HTMLButtonElement>('#connect'); button.disabled = true; try { render(await invoke<Snapshot>('phone_start', {address: $<HTMLSelectElement>('#interface').value})); $('#network-status').textContent = '連線已啟動。防火牆請允許信任的私人網路。選錯網路時可切換後按「套用」。'; button.textContent = '套用'; } catch(e) { $('#network-status').textContent = String(e); toast(String(e)); } finally {button.disabled = false;} }
 $('#connect').onclick = () => void connect();
 if (interfaces.length) { await connect(); await refresh(); } else { $('#room-status').textContent = '請先連接 Wi-Fi／乙太網路，再重新開啟'; }
}
async function downloadFile(fileId: string) {
 $('#cancel-download').hidden = false; $('#transfer-status').textContent = '準備下載…';
 try { await invoke('download_file', {fileId}); $('#transfer-status').textContent = '✓ 下載完成，已存入下載／PocketDrop'; toast('✓ 下載完成'); }
 catch (e) { $('#transfer-status').textContent = String(e); }
 finally { $('#cancel-download').hidden = true; }
}
$('#cancel-download').onclick = () => void invoke('cancel_download');
$('#downloads').onclick = () => void invoke('show_downloads').catch(e => toast(String(e)));
$('#copy-code').onclick = () => void navigator.clipboard.writeText($('#pair-code').textContent ?? '').then(() => toast('已複製驗證碼'));
$('#leave').onclick = () => { if (confirm('返回自己的 Room？這台電腦需重新配對才能再加入目前 Room。')) void invoke('leave_room').then(() => { online = true; dirty = false; void refresh(); }); };
let scanner: IScannerControls | undefined;
function stopScanner() { scanner?.stop(); scanner = undefined; $<HTMLVideoElement>('#camera').hidden = true; }
async function scanRooms() {
 const rooms = await invoke<{device_id:string; endpoint:string; name:string}[]>('nearby_rooms');
 $('#nearby').replaceChildren(...rooms.map(r => { const o = document.createElement('option'); o.value=r.endpoint; o.textContent=r.name; return o; }));
 $('#join-status').textContent = rooms.length ? '選擇畫面上相同編號的電腦，輸入驗證碼。' : '正在尋找。請讓另一台電腦開啟 PocketDrop，稍後按重新尋找。';
}
$('#join').onclick = () => { $<HTMLDialogElement>('#join-dialog').showModal(); void scanRooms(); };
$('#close-join').onclick = () => { stopScanner(); $<HTMLDialogElement>('#join-dialog').close(); };
$('#join-dialog').addEventListener('close', stopScanner);
$('#scan-rooms').onclick = () => void scanRooms().catch(e => toast(String(e)));
async function joinAction(command: string, args: Record<string,string>) {
 const button = $<HTMLButtonElement>('#join-code'); button.disabled = true; $('#join-status').textContent = '正在驗證裝置身分…';
 try { await invoke(command,args); stopScanner(); $<HTMLDialogElement>('#join-dialog').close(); dirty = false; online = true; await refresh(); toast('✓ 已加入 Room，之後會自動連線'); }
 catch(e) { $('#join-status').textContent = String(e); }
 finally { button.disabled = false; }
}
$('#join-code').onclick = () => void joinAction('join_code',{address:$<HTMLSelectElement>('#nearby').value,code:$<HTMLInputElement>('#verification').value.trim()});
$('#scan-camera').onclick = async () => { try { stopScanner(); $<HTMLVideoElement>('#camera').hidden = false; scanner = await new BrowserQRCodeReader().decodeFromVideoDevice(undefined,$<HTMLVideoElement>('#camera'),(result,_error,controls) => { if (result) { controls.stop(); void joinAction('join_qr',{qr:result.getText()}); } }); } catch { $('#join-status').textContent = '無法使用攝影機；可選取 QR 圖片或使用驗證碼。'; } };
$<HTMLInputElement>('#qr-image').onchange = async e => { const file=(e.target as HTMLInputElement).files?.[0]; if(!file)return; if(file.size>10*1024*1024){toast('QR 圖片請小於 10 MB');return;} const url=URL.createObjectURL(file); try { const result=await new BrowserQRCodeReader().decodeFromImageUrl(url); await joinAction('join_qr',{qr:result.getText()}); } catch { $('#join-status').textContent = '圖片中找不到 PocketDrop QR Code'; } finally { URL.revokeObjectURL(url); } };
void init().catch(e => toast(`初始化失敗：${String(e)}`));
