import { isTauri } from '@tauri-apps/api/core';
import { check, Update } from '@tauri-apps/plugin-updater';

export function initializeUpdates(beforeInstall: () => void) {
  const settings = document.querySelector('.settings')!;
  settings.insertAdjacentHTML('beforeend', `<div class="control-row"><button id="check-update">檢查更新</button></div><p class="hint" id="update-status" role="status">啟動時自動檢查新版；更新檢查需要網際網路。</p>`);
  document.querySelector('header')!.insertAdjacentHTML('afterend', '<button id="update-available" class="primary" hidden></button>');
  document.body.insertAdjacentHTML('beforeend', `<dialog id="update-dialog" aria-labelledby="update-title"><h2 id="update-title">PocketDrop 有新版本</h2><pre id="update-notes"></pre><p class="hint">按「是，更新」後下載並驗證更新包，再關閉程式安裝。Room 會暫時離線，配對資料會保留，未分享文字會保存為草稿。請先完成檔案傳輸。</p><p id="update-progress" role="status" aria-live="polite"></p><div class="control-row"><button id="update-later">稍後</button><button id="update-install" class="primary">是，更新</button></div></dialog>`);
  const find = <T extends HTMLElement>(id: string) => document.querySelector<T>(id)!;
  const dialog = find<HTMLDialogElement>('#update-dialog');
  const manual = find<HTMLButtonElement>('#check-update');
  const banner = find<HTMLButtonElement>('#update-available');
  const install = find<HTMLButtonElement>('#update-install');
  const later = find<HTMLButtonElement>('#update-later');
  const status = find('#update-status');
  const progress = find('#update-progress');
  let pending: Update | null = null;
  let busy = false;
  const show = () => { if (pending && !dialog.open) dialog.showModal(); };
  banner.onclick = show;
  later.onclick = () => { if (!busy) dialog.close(); };
  dialog.addEventListener('cancel', e => { if (busy) e.preventDefault(); });
  async function checkNow() {
    if (busy) return;
    if (!isTauri()) { status.textContent = '請在 Windows App 檢查更新。'; return; }
    busy = true; manual.disabled = true;
    status.textContent = '正在檢查更新…';
    try {
      await pending?.close(); pending = null; banner.hidden = true;
      pending = await check({ timeout: 12000 });
      if (!pending) { status.textContent = '目前已是最新版本。'; return; }
      const title = `PocketDrop ${pending.currentVersion} → ${pending.version}`;
      find('#update-title').textContent = title;
      find('#update-notes').textContent = (pending.body || '此版本提供功能更新與修正。').slice(0, 12000);
      progress.textContent = '';
      status.textContent = `有新版本 ${pending.version}，可選擇更新。`;
      banner.textContent = `有新版本 ${pending.version} · 查看更新`;
      banner.hidden = false;
      if (!document.querySelector('dialog[open]')) show();
    } catch {
      status.textContent = '暫時無法檢查更新，請確認網路後重試。LAN 分享仍可使用。';
    } finally { busy = false; manual.disabled = false; }
  }
  install.onclick = async () => {
    if (busy || !pending) return;
    busy = true; install.disabled = true; later.disabled = true; manual.disabled = true;
    let downloaded = 0, total = 0;
    try {
      beforeInstall();
      progress.textContent = '準備下載更新…';
      await pending.download(event => {
        if (event.event === 'Started') total = event.data.contentLength || 0;
        if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          progress.textContent = total ? `下載中 ${Math.min(100, Math.floor(downloaded / total * 100))}%` : `已下載 ${(downloaded / 1048576).toFixed(1)} MB`;
        }
        if (event.event === 'Finished') progress.textContent = '正在驗證更新包簽章…';
      }, { timeout: 180000 });
      beforeInstall();
      progress.textContent = '驗證通過，正在啟動安裝，完成後會重新開啟。';
      await pending.install();
    } catch (error) {
      progress.textContent = error instanceof Error && error.message.startsWith('更新前：')
        ? error.message : '更新未完成：下載、簽章驗證或安裝啟動失敗。請稍後重試；目前版本仍可使用。';
    } finally { busy = false; install.disabled = false; later.disabled = false; manual.disabled = false; }
  };
  manual.onclick = () => void checkNow();
  if (isTauri()) void checkNow();
}
