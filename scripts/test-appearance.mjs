import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import ts from 'typescript';
const code=ts.transpileModule(fs.readFileSync('src/appearance.ts','utf8').replace(/^import .*;\r?\n/m,'').replace('export function','function'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
function app(saved, unsupported=false) {
 const calls=[]; const select={value:'',disabled:false,scrollIntoView(){},focus(){}}; const status={textContent:''}; const shortcut={}; const body={dataset:{}};
 const storage=new Map(saved?[['pocketdrop.appearance',saved]]:[]);
 const context={document:{body,querySelector:s=>({'#appearance':select,'#material-status':status,'#appearance-shortcut':shortcut,'.settings':{}}[s])},localStorage:{getItem:k=>storage.get(k),setItem:(k,v)=>storage.set(k,v)},isTauri:()=>true,invoke:async(cmd,args)=>{calls.push(args.mode);if(unsupported&&args.mode==='acrylic')throw Error('unsupported DWM');}};
 vm.createContext(context);vm.runInContext(code+';globalThis.apply=initializeAppearance()',context);
 return {context,select,status,body,storage,calls};
}
let a=app();await a.context.apply();assert.equal(a.body.dataset.appearance,'dark');assert.deepEqual(a.calls,['off']);
a=app('light');await a.context.apply();assert.equal(a.body.dataset.appearance,'light');assert.equal(a.storage.get('pocketdrop.appearance'),'light');
a=app('glass',true);await a.context.apply();assert.equal(a.body.dataset.appearance,'dark');assert.equal(a.select.value,'dark');assert.equal(a.storage.get('pocketdrop.appearance'),'dark');assert.deepEqual(a.calls,['acrylic','off']);assert.match(a.status.textContent,/清晰外觀/);
a=app('glass');await a.context.apply();assert.equal(a.body.dataset.appearance,'glass');
a=app('invalid');await a.context.apply();assert.equal(a.body.dataset.appearance,'dark');
function luminance(hex){return hex.match(/../g).map(x=>parseInt(x,16)/255).map(x=>x<=.04045?x/12.92:((x+.055)/1.055)**2.4).reduce((s,x,i)=>s+x*[.2126,.7152,.0722][i],0)}
let minimum=100;for(const [ink,muted,bgs] of [['f1f5fa','b8c6d5',['111c2b','1b2a3b','142131']],['162d3d','4a6070',['eef3f5','ffffff','f2f6f8']]])for(const fg of [ink,muted])for(const bg of bgs){const a=luminance(fg),b=luminance(bg);const ratio=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);minimum=Math.min(minimum,ratio);assert.ok(ratio>=4.5)}
console.log('PASS: default, restored light, unsupported glass fallback and persistence, supported glass, invalid setting; text contrast minimum '+minimum.toFixed(2)+':1');
