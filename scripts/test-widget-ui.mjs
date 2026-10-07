import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const {chromium}=await import(pathToFileURL(resolve(process.argv[2] ?? '.test-runs/ui/node_modules/playwright/index.mjs')).href);
import {readFile,readdir,mkdir} from 'node:fs/promises';
import assert from 'node:assert/strict';
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
 const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.setContent('<html><body><div id="app"></div></body></html>');
 for(const f of await readdir('dist/assets')){if(f.endsWith('.css'))await page.addStyleTag({path:'dist/assets/'+f});if(f.endsWith('.js'))await page.addScriptTag({content:await readFile('dist/assets/'+f,'utf8'),type:'module'});}
 await page.waitForSelector('#text');
 for(const width of [320,400,480,1000])for(const theme of ['dark','light','glass']){
  await page.setViewportSize({width,height:860});await page.evaluate(t=>document.body.dataset.appearance=t,theme);
  const input=page.locator('#text');await input.fill('PocketDrop 文字測試 '.repeat(150));
  await page.waitForFunction(()=>document.querySelector('#text').getBoundingClientRect().height>180);
  const height=await input.evaluate(e=>e.getBoundingClientRect().height);assert.ok(height<=559);
  await input.evaluate(e=>e.setSelectionRange(5,50));await page.evaluate(()=>window.dispatchEvent(new Event('resize')));await page.waitForTimeout(40);
  assert.deepEqual(await input.evaluate(e=>[e.selectionStart,e.selectionEnd]),[5,50]);
  await input.fill('短文字');await page.waitForFunction(()=>document.querySelector('#text').getBoundingClientRect().height===180);
  assert.equal(await page.locator('.widget').evaluate(e=>e.scrollWidth>e.clientWidth+1),false,`${width}/${theme} overflow`);
 }
 await page.setViewportSize({width:480,height:860});await page.evaluate(()=>document.body.dataset.appearance='dark');
 await page.locator('[data-section=".settings"]').click();assert.equal(await page.locator('.settings').evaluate(e=>e.open),true);
 await page.locator('.widget').evaluate(e=>e.scrollTop=0);await page.locator('[data-section=".text-card"]').click();assert.equal(await page.locator('#text').evaluate(e=>e===document.activeElement),true);
 await page.locator('.widget').evaluate(e=>e.scrollTop=0);await mkdir('evidence/ui-1.0.6',{recursive:true});
 for(const theme of ['dark','light']){await page.evaluate(t=>document.body.dataset.appearance=t,theme);await page.waitForTimeout(200);await page.screenshot({path:`evidence/ui-1.0.6/${theme}.png`});}
 await page.setViewportSize({width:1000,height:860});await page.screenshot({path:'evidence/ui-1.0.6/wide.png'});
 await page.evaluate(()=>document.querySelector('#pair-dialog').showModal());await page.locator('#close-pair').click();assert.equal(await page.locator('#pair-dialog').evaluate(e=>e.open),false);
 assert.deepEqual(errors,[]);console.log('PASS widget: 12 viewport/theme combinations, text sizing and selection, no overflow, navigation, dialog close');
} finally {await browser.close();}
