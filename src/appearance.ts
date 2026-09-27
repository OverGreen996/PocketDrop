import { invoke, isTauri } from '@tauri-apps/api/core';

type Appearance = 'dark' | 'light' | 'glass';
const key = 'pocketdrop.appearance';

export function initializeAppearance() {
  const select = document.querySelector<HTMLSelectElement>('#appearance')!;
  const status = document.querySelector<HTMLElement>('#material-status')!;
  let choice: Appearance = 'dark';
  try {
    const saved = localStorage.getItem(key);
    if (saved === 'dark' || saved === 'light' || saved === 'glass') choice = saved;
  } catch { /* Storage restrictions must not prevent a readable window. */ }
  select.value = choice;
  document.body.dataset.appearance = choice === 'glass' ? 'dark' : choice;
  status.textContent = '清晰外觀使用不透明背景，不受桌布或玻璃支援影響。';
  document.querySelector<HTMLButtonElement>('#appearance-shortcut')!.onclick = () => {
    const settings = document.querySelector<HTMLDetailsElement>('.settings')!;
    settings.open = true;
    select.scrollIntoView({ block: 'center' });
    select.focus({ preventScroll: true });
  };
  async function apply() {
    select.disabled = true;
    // Paint opaque before changing the compositor, including glass failure.
    document.body.dataset.appearance = choice === 'light' ? 'light' : 'dark';
    try {
      if (choice === 'glass') {
        if (!isTauri()) throw new Error('請在 Windows App 使用玻璃外觀');
        await invoke('set_material', { mode: 'acrylic' });
        document.body.dataset.appearance = 'glass';
        status.textContent = '原生玻璃已啟用。文字不清楚時可改選清晰外觀。';
      } else {
        if (isTauri()) await invoke('set_material', { mode: 'off' });
        status.textContent = '清晰外觀使用不透明背景，不受桌布或玻璃支援影響。';
      }
    } catch {
      choice = document.body.dataset.appearance === 'light' ? 'light' : 'dark';
      select.value = choice;
      status.textContent = '此環境無法套用原生材質，已使用清晰外觀。玻璃需要 Windows 11 22H2 以上。';
      if (isTauri()) await invoke('set_material', { mode: 'off' }).catch(() => {});
    } finally {
      try { localStorage.setItem(key, choice); }
      catch { status.textContent += ' 無法保存設定，下次開啟將使用預設外觀。'; }
      select.disabled = false;
    }
  }
  select.onchange = () => { choice = select.value as Appearance; void apply(); };
  return apply;
}
