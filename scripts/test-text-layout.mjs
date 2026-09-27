// Run against a built frontend served on localhost. Pass the Playwright ESM path.
// node scripts/test-text-layout.mjs <path-to-playwright/index.mjs> [base-url]
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const { chromium } = await import(pathToFileURL(resolve(process.argv[2] ?? 'node_modules/playwright/index.mjs')).href);
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
 const page = await browser.newPage();
 await page.goto(process.argv[3] ?? 'http://127.0.0.1:1430');
 for (const width of [320, 360, 361, 440, 1000]) {
  await page.setViewportSize({ width, height: 740 });
  for (const theme of ['dark', 'light', 'glass']) {
   await page.evaluate(theme => { document.body.dataset.appearance = theme; }, theme);
   const text = page.locator('#text');
   await text.fill('HhHRRHhHRR'.repeat(150));
   await page.waitForFunction(() => {
    const e = document.querySelector('#text');
    return e.getBoundingClientRect().height > 180 && Math.abs(e.getBoundingClientRect().height - parseFloat(e.style.height)) < 1;
   }, null, { timeout: 2000 });
   const h = await text.evaluate(e => e.getBoundingClientRect().height);
   assert.ok(h <= 481, `${width}/${theme}: bounded height`);
   assert.equal((await text.inputValue()).length, 1500);
   await text.evaluate(e => { e.focus(); e.setSelectionRange(5, 50); });
   await page.evaluate(() => window.dispatchEvent(new Event('resize')));
   await page.waitForTimeout(50);
   assert.deepEqual(await text.evaluate(e => [e.selectionStart, e.selectionEnd]), [5, 50]);
   await text.fill('短文字');
   await page.waitForFunction(() => document.querySelector('#text').getBoundingClientRect().height === 180);
   console.log(`PASS ${width}px ${theme}: expansion ${h}px, shrink 180px, full content and selection retained`);
  }
 }
} finally { await browser.close(); }
