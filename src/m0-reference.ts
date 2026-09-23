import './style.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const icon = (name: 'box' | 'text' | 'arrow') => ({
  box: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4"><path d="m3 7 9-5 9 5v10l-9 5-9-5V7Z M3 7l9 5 9-5 M12 12v10 M7 4.8l9 5"/></svg>',
  text: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 5h16M12 5v15M8 20h8"/></svg>',
  arrow: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 3v13m-5-5 5 5 5-5M4 17v4h16v-4"/></svg>',
})[name];

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
<main class="widget">
  <header><div class="brand" id="drag-handle" title="拖曳移動 Widget"><span class="brand-icon">${icon('box')}</span><div><h1>PocketDrop<span class="version">M0</span></h1><p><span class="status-dot"></span>本機技術測試</p></div></div><div class="window-actions"><button id="pin" title="永遠置頂" aria-label="永遠置頂" aria-pressed="false">◇</button><button id="minimize" title="最小化" aria-label="最小化">−</button><button id="close" title="關閉" aria-label="關閉">×</button></div></header>
  <div class="intro"><span class="eyebrow">YOUR LITTLE SHARED SPACE</span><p>貼進去，丟進去，拿出來。</p></div>
  <section class="card text-card"><div class="section-head"><h2>${icon('text')} Shared Text</h2><span class="tag">本機示意</span></div><textarea id="text" aria-label="本機文字測試區" spellcheck="false" maxlength="10000">靈感先放這裡。\n讓不同裝置之間，少一點距離。</textarea><div class="card-foot"><span>字體與選取測試 · 尚未同步</span><button id="copy">複製文字 <span aria-hidden="true">↗</span></button></div></section>
  <section class="card files-card" id="files"><div class="section-head"><h2>${icon('box')} Shared Files</h2><span class="tag" id="file-count">0 個檔案</span></div><div class="drop-zone"><span class="drop-icon">${icon('arrow')}</span><strong id="drop-title">把檔案放到這裡</strong><span id="drop-subtitle">支援多檔拖入，格式不限</span></div><ul id="file-list" aria-label="本機拖放結果"></ul><div class="card-foot"><span>僅顯示檔名與大小 · 不會上傳</span><button id="clear-files">清除</button></div></section>
  <section class="diagnostics"><div class="section-head"><h2>實機測試</h2><button id="backdrop" class="test-label">開啟外部背景測試板 ↗</button></div><div class="control-row"><label for="material">背景材質</label><select id="material"><option value="acrylic">原生 Acrylic</option><option value="off">關閉模糊（對照）</option></select><button id="contrast" title="切換深色／亮色文字" aria-label="切換深色／亮色文字">◐</button></div><p class="hint" id="material-status">正在初始化原生材質…</p><div class="control-row"><label for="interface">探索介面</label><select id="interface" aria-label="mDNS LAN 測試介面"><option value="">讀取中…</option></select><button id="discovery">開始</button></div><p class="hint" id="network-status">只探索附近測試裝置，尚未配對。</p><ul id="peers" aria-label="探索到的測試裝置"></ul></section>
  <footer><span><span class="status-dot amber"></span> M0 可行性原型</span><span>LAN ONLY · NO CLOUD</span></footer>
  <div id="toast" role="status" aria-live="polite"></div>
</main>`;

const $ = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector)!;
let toastTimer: number;
function toast(message: string) {
  $('#toast').textContent = message;
  $('#toast').classList.add('show');
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => $('#toast').classList.remove('show'), 4200);
}
const formatSize = (bytes: number) => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const index = bytes ? Math.min(4, Math.floor(Math.log(bytes) / Math.log(1024))) : 0;
  return `${(bytes / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`;
};
interface FileMeta { name: string; size: number }
let files: FileMeta[] = [];
function renderFiles() {
  $('#file-count').textContent = `${files.length} 個檔案`;
  $('#file-list').replaceChildren(...files.map(file => {
    const li = document.createElement('li');
    const name = document.createElement('span'); name.textContent = file.name; name.title = file.name;
    const size = document.createElement('small'); size.textContent = formatSize(file.size);
    li.append(name, size); return li;
  }));
  $('#files').classList.toggle('has-files', files.length > 0);
}
$('#clear-files').onclick = () => { files = []; renderFiles(); };
$('#copy').onclick = async () => {
  try { await navigator.clipboard.writeText($<HTMLTextAreaElement>('#text').value); toast('✓ 已複製文字'); }
  catch { toast('複製失敗，請選取文字後按 Ctrl+C'); }
};
$('#contrast').onclick = () => document.body.classList.toggle('light-ink');

async function init() {
  if (!isTauri()) {
    $('#material-status').textContent = '瀏覽器預覽無法驗證桌面玻璃；請啟動 Windows 執行檔。';
    $('#network-status').textContent = '瀏覽器预覽不提供原生拖放或 mDNS。';
    document.querySelectorAll<HTMLButtonElement | HTMLSelectElement>('.window-actions button, .diagnostics select, #discovery').forEach(e => e.disabled = true);
    return;
  }
  const win = getCurrentWindow();
  await invoke('initialize_window');
  $('#backdrop').onclick = () => void invoke('open_backdrop').catch(error => toast(String(error)));
  $('#drag-handle').onpointerdown = e => { if (e.button === 0) void win.startDragging().catch(() => toast('視窗拖曳失敗')); };
  $('#close').onclick = () => void win.close().catch(() => toast('無法關閉'));
  $('#minimize').onclick = () => void win.minimize().catch(() => toast('無法最小化'));
  let pinned = false;
  $('#pin').onclick = async () => {
    try { await win.setAlwaysOnTop(!pinned); pinned = !pinned; $('#pin').setAttribute('aria-pressed', String(pinned)); toast(pinned ? '已永遠置頂' : '已回到一般層級'); }
    catch { toast('無法變更置頂狀態'); }
  };
  await listen<boolean>('drop-hover', ({ payload }) => {
    document.body.classList.toggle('drag-over', payload);
    $('#drop-title').textContent = payload ? '放開以加入' : '把檔案放到這裡';
  });
  await listen<{ accepted: FileMeta[]; rejected: { name: string; reason: string }[] }>('drop-result', ({ payload }) => {
    files = [...payload.accepted, ...files].slice(0, 100); renderFiles();
    toast(payload.rejected.length ? `${payload.accepted.length} 個檔案已加入測試區；${payload.rejected[0].reason}` : `✓ ${payload.accepted.length} 個檔案已加入測試區`);
  });
  await listen<string>('m0-error', ({ payload }) => { toast(payload); $('#network-status').textContent = payload; });
  async function material() {
    const mode = $<HTMLSelectElement>('#material').value;
    try { $('#material-status').textContent = await invoke<string>('set_material', { mode }); }
    catch (error) { $('#material-status').textContent = `原生材質失敗：${String(error)}。不符合玻璃驗收。`; }
  }
  $<HTMLSelectElement>('#material').onchange = () => void material();
  await material();
  const interfaces = await invoke<{ name: string; address: string }[]>('interfaces');
  $('#interface').replaceChildren(...interfaces.map(i => {
    const option = document.createElement('option'); option.value = i.address; option.textContent = `${i.name} · ${i.address}`; return option;
  }));
  if (!interfaces.length) { $('#network-status').textContent = '沒有可用 LAN IPv4 介面，請連接 Wi-Fi／乙太網路。'; $<HTMLButtonElement>('#discovery').disabled = true; }
  interface Peer { kind: string; session: string; id: string; fullname: string; addresses: string[]; port: number }
  const peers = new Map<string, Peer>();
  let session: string | null = null;
  await listen<Peer>('discovery-peer', ({ payload: p }) => {
    if (p.session !== session) return;
    if (p.kind === 'removed') peers.delete(p.fullname);
    else if (peers.size < 100 || peers.has(p.fullname)) peers.set(p.fullname, p);
    $('#peers').replaceChildren(...[...peers.values()].map(p => {
      const li = document.createElement('li'); li.textContent = `測試裝置 ${p.id.slice(0, 8)} · 未驗證`; li.title = `${p.addresses.join(', ')}:${p.port}`; return li;
    }));
    $('#network-status').textContent = `探索中 · ${peers.size} 台其他測試裝置 · 未配對`;
  });
  $('#discovery').onclick = async () => {
    const button = $<HTMLButtonElement>('#discovery'); button.disabled = true;
    try {
      if (session) { await invoke('stop_discovery'); session = null; peers.clear(); $('#peers').replaceChildren(); }
      else { session = await invoke<string>('start_discovery', { address: $<HTMLSelectElement>('#interface').value }); toast('若 Windows 詢問防火牆，僅允許信任的私人網路，用於尋找附近測試裝置。'); }
      button.textContent = session ? '停止' : '開始';
      $<HTMLSelectElement>('#interface').disabled = !!session;
      $('#network-status').textContent = session ? `探索中 · 本機 ${session.slice(0, 8)} · 尚未配對` : '探索已停止。';
    } catch (error) { toast(`探索失敗：${String(error)}`); }
    finally { button.disabled = false; }
  };
}
void init().catch(error => toast(`初始化失敗：${String(error)}`));
