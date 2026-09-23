// Headless loopback test. All credentials are ephemeral, confined to ignored .test-runs.
import {spawn} from 'node:child_process';
import {mkdir,readFile,writeFile,readdir,unlink} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import https from 'node:https';
import {createHash,randomUUID,randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
import {DatabaseSync} from 'node:sqlite';
const root=resolve('.test-runs',randomUUID());await mkdir(root,{recursive:true});
const executable=resolve('src-tauri/target/release/pocketdrop-m0.exe');
const process=spawn(executable,['--phone-smoke',root],{windowsHide:true,stdio:'ignore'});
const results=[];
let agent;
try {
 let profile;
 for(let i=0;i<100;i++){try{profile=JSON.parse(await readFile(join(root,'bootstrap.json'),'utf8'));break;}catch{await new Promise(r=>setTimeout(r,100));}}
 assert.ok(profile,'Headless server started');
 const ca=await readFile(join(root,'identity-cert.pem'));
 agent=new https.Agent({ca,keepAlive:true,checkServerIdentity:(_,cert)=>createHash('sha256').update(cert.raw).digest('hex')===profile.certificate_sha256?undefined:new Error('pin mismatch')});
 let credential;
 function request(path,{method='GET',data,headers={},auth=true}={}){
  const body=data===undefined?undefined:Buffer.isBuffer(data)?data:Buffer.from(JSON.stringify(data));
  return new Promise((ok,bad)=>{const req=https.request(profile.endpoint+path,{agent,method,headers:{...(body?{'content-length':body.length,'content-type':Buffer.isBuffer(data)?'application/octet-stream':'application/json'}:{}),...(auth&&credential?{authorization:`Bearer ${credential}`,'x-pocketdrop-room':profile.room_id}:{}),...headers}},res=>{const chunks=[];res.on('data',c=>chunks.push(c));res.on('end',()=>ok({status:res.statusCode,headers:res.headers,body:Buffer.concat(chunks)}));res.on('error',bad);});req.on('error',bad);req.setTimeout(10000,()=>req.destroy(new Error('timeout')));req.end(body);});
 }
 const check=(name)=>results.push({name,result:'pass'});
 assert.equal((await request('/v1/state')).status,401);assert.equal((await request('/v1/files/anything')).status,401);check('Unauthenticated state and files denied');
 const pairing={token:profile.token,device_id:randomUUID(),name:'Android integration test',public_key:randomBytes(64).toString('base64')};
 let response=await request('/v1/pair',{method:'POST',data:pairing,auth:false});assert.equal(response.status,200);credential=JSON.parse(response.body).credential;
 assert.equal((await request('/v1/pair',{method:'POST',data:pairing,auth:false})).status,401);check('TLS-pinned pairing and one-use invitation');
 assert.equal((await request('/v1/state',{headers:{'x-pocketdrop-room':randomUUID()}})).status,403);check('Wrong Room rejected');
 const wrongPin=new https.Agent({ca,checkServerIdentity:()=>new Error('intentional pin mismatch')});
 await assert.rejects(new Promise((ok,bad)=>{https.get(profile.endpoint+'/v1/state',{agent:wrongPin},ok).on('error',bad);}),/pin mismatch/);wrongPin.destroy();check('Mismatched TLS pin rejected');
 const wsHeaders={connection:'Upgrade',upgrade:'websocket','sec-websocket-version':'13','sec-websocket-key':randomBytes(16).toString('base64')};
 assert.equal((await request('/v1/events',{headers:wsHeaders,auth:false})).status,401);
 const ws=await new Promise((ok,bad)=>{const req=https.request(profile.endpoint+'/v1/events',{agent,headers:{...wsHeaders,authorization:`Bearer ${credential}`,'x-pocketdrop-room':profile.room_id}});req.on('upgrade',(_,socket)=>ok(socket));req.on('response',r=>bad(new Error('WebSocket refused '+r.statusCode)));req.on('error',bad);req.end();});
 let wsData='';ws.on('data',b=>wsData+=b.toString());ws.on('error',()=>{});
 response=await request('/v1/text',{method:'PUT',data:{content:'手機 ↔ 電腦\nSTL、PDF、Prompt'}});assert.equal(response.status,204);
 let state=JSON.parse((await request('/v1/state')).body);assert.equal(state.text,'手機 ↔ 電腦\nSTL、PDF、Prompt');assert.equal(state.revision,1);
 for(let i=0;i<30&&!wsData.includes('refresh');i++)await new Promise(r=>setTimeout(r,100));assert.ok(wsData.includes('refresh'));ws.destroy();check('Authenticated WebSocket delivers refresh; unauthenticated upgrade denied');
 assert.equal((await request('/v1/text',{method:'PUT',data:{content:'x'.repeat(32769)}})).status,413);check('Unicode text persists and oversized text rejected');
 const bytes=randomBytes(2*1024*1024+123);
 response=await request('/v1/files?name='+encodeURIComponent('模型.stl'),{method:'PUT',data:bytes,headers:{'x-file-size':bytes.length}});assert.equal(response.status,201);
 state=JSON.parse((await request('/v1/state')).body);assert.equal(state.files.length,1);assert.equal(state.files[0].name,'模型.stl');assert.ok(!JSON.stringify(state).includes('local_path'));assert.ok(!JSON.stringify(state).includes(root));
 const id=state.files[0].file_id;response=await request('/v1/files/'+id);assert.equal(response.status,200);assert.deepEqual(response.body,bytes);check('2 MiB upload/download exact bytes; no source paths in metadata');
 response=await request('/v1/files/'+id,{headers:{range:'bytes=100-299'}});assert.equal(response.status,206);assert.deepEqual(response.body,bytes.subarray(100,300));assert.equal(response.headers['content-range'],`bytes 100-299/${bytes.length}`);
 assert.equal((await request('/v1/files/'+id,{headers:{range:'bytes=4-1'}})).status,416);check('HTTP Range bounds and contents');
 response=await request('/v1/files?name=upload.part',{method:'PUT',data:Buffer.alloc(0),headers:{'x-file-size':'0'}});assert.equal(response.status,201);
 state=JSON.parse((await request('/v1/state')).body);const empty=state.files.find(f=>f.name==='upload.part');assert.equal(empty.available,true);assert.equal((await request('/v1/files/'+empty.file_id)).body.length,0);check('Zero-byte and upload.part filename regression');
 for(const name of ['../outside.txt','CON.txt','C:\\Users\\x.txt','bad/name','tail.']) assert.equal((await request('/v1/files?name='+encodeURIComponent(name),{method:'PUT',data:Buffer.from('x'),headers:{'x-file-size':'1'}})).status,400);
 assert.equal((await request('/v1/files?name=wrong-size.txt',{method:'PUT',data:Buffer.from('abc'),headers:{'x-file-size':'2'}})).status,413);check('Traversal, Windows reserved names, size mismatch rejected');
 for(let i=0;i<20;i++){state=JSON.parse((await request('/v1/state')).body);if(state.files.find(f=>f.file_id===id).sha256)break;await new Promise(r=>setTimeout(r,100));}
 assert.equal(state.files.find(f=>f.file_id===id).sha256,createHash('sha256').update(bytes).digest('hex'));check('Background SHA-256 integrity metadata');
 const dirs=await readdir(join(root,'inbox'),{withFileTypes:true});for(const d of dirs){if(d.isDirectory()){try{await unlink(join(root,'inbox',d.name,'模型.stl'));}catch{}}}
 assert.equal((await request('/v1/files/'+id)).status,410);state=JSON.parse((await request('/v1/state')).body);assert.equal(state.files.find(f=>f.file_id===id).available,false);check('Missing source becomes unavailable');
 // Test the same persisted membership flag used by the local-only revoke command.
 const db=new DatabaseSync(join(root,'phone-trial.sqlite'));db.prepare('UPDATE members SET revoked=1 WHERE id=?').run(pairing.device_id);db.close();
 assert.equal((await request('/v1/state')).status,401);assert.equal((await request('/v1/text',{method:'PUT',data:{content:'denied'}})).status,401);assert.equal((await request('/v1/files/'+empty.file_id)).status,401);check('Revoked credential rejected for text, metadata and download');
 await mkdir('evidence/phone-trial',{recursive:true});await writeFile('evidence/phone-trial/api-smoke.json',JSON.stringify({at:new Date().toISOString(),environment:'Windows loopback HTTPS; not a physical Android test',results},null,2));
 console.log(`${results.length} API scenarios passed`);
}finally{agent?.destroy();process.kill();}
