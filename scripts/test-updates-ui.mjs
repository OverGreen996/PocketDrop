import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const { chromium } = await import(pathToFileURL(resolve(process.argv[2] ?? 'node_modules/playwright/index.mjs')).href);
import ts from 'typescript';
import fs from 'node:fs';
import assert from 'node:assert/strict';
const source=ts.transpileModule(fs.readFileSync('src/updates.ts','utf8').replace(/^import .*;\r?\n/gm,'').replace('export function','function'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
 for(const scenario of ['available','latest','offline','bad-signature','draft-failed']) {
  const page=await browser.newPage();
  await page.setContent('<header></header><details class="settings" open></details>');
  await page.evaluate(({source,scenario})=>{
   window.calls=[];
   window.isTauri=()=>true;
   window.check=async()=>{
    window.calls.push('check');
    if(scenario==='offline')throw Error('network');
    if(scenario==='latest')return null;
    return {version:'1.0.6',currentVersion:'1.0.5',body:'<img src=x onerror=alert(1)>',close:async()=>window.calls.push('close'),download:async callback=>{
     window.calls.push('download');callback({event:'Started',data:{contentLength:100}});callback({event:'Progress',data:{chunkLength:100}});callback({event:'Finished'});
     if(scenario==='bad-signature')throw Error('signature invalid');
    },install:async()=>window.calls.push('install')};
   };
   new Function(source+';initializeUpdates(() => { window.calls.push("save"); if("'+scenario+'" === "draft-failed") throw new Error("更新前：無法保存文字草稿"); });')();
  },{source,scenario});
  if(['latest','offline'].includes(scenario)) {
   await page.waitForFunction(()=>!document.querySelector('#check-update').disabled);
   assert.equal(await page.locator('dialog[open]').count(),0);
   assert.match(await page.locator('#update-status').innerText(),scenario==='latest'?/最新/:/無法/);
   await page.locator('#check-update').click();await page.waitForTimeout(50);
   assert.deepEqual(await page.evaluate(()=>window.calls),['check','check']);
  }else{
   await page.waitForSelector('#update-dialog[open]');
   assert.deepEqual(await page.evaluate(()=>window.calls),['check']);
   assert.equal(await page.locator('#update-notes img').count(),0);
   await page.locator('#update-later').click();assert.deepEqual(await page.evaluate(()=>window.calls),['check']);
   await page.locator('#update-available').click();await page.locator('#update-install').click();
   await page.waitForFunction(()=>!document.querySelector('#update-install').disabled);
   const calls=await page.evaluate(()=>window.calls);
   if(scenario==='available')assert.deepEqual(calls,['check','save','download','save','install']);
   else {assert.ok(!calls.includes('install'));assert.match(await page.locator('#update-progress').innerText(),/未完成|更新前/);}
  }
  console.log('PASS updater UI:',scenario);await page.close();
 }
}finally{await browser.close();}
