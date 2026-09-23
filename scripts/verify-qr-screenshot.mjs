// Decode the actual Windows screenshot without printing the short-lived invitation.
import {readFile,writeFile} from 'node:fs/promises';
import jpeg from 'jpeg-js';
import jsQR from 'jsqr';
import assert from 'node:assert/strict';
const png=jpeg.decode(await readFile('evidence/phone-trial/windows-qr.jpg'),{useTArray:true});
const qr=jsQR(new Uint8ClampedArray(png.data),png.width,png.height);
assert.ok(qr,'Complete pairing code must be decodable from the actual window');
const data=JSON.parse(qr.data);
assert.equal(data.app,'PocketDrop');assert.equal(data.mode,'phone-trial');assert.equal(data.protocol_version,1);
assert.match(data.token,/^[A-Za-z0-9_-]{43}$/);assert.match(data.certificate_sha256,/^[a-f0-9]{64}$/);
await writeFile('evidence/phone-trial/qr-result.json',JSON.stringify({result:'pass',source:'actual Windows screenshot',image:{width:png.width,height:png.height},invitation:'not retained in test output; invalidated by closing test app'},null,2));
console.log('Actual Windows QR screenshot decoded successfully; required pairing fields present');
