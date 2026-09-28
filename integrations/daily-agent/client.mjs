import https from 'node:https';
import tls from 'node:tls';
import { createHash, generateKeyPairSync, randomUUID } from 'node:crypto';
import { Bonjour } from 'bonjour-service';

export function endpoint(value) {
  const u = new URL(value);
  const p = u.hostname.split('.').map(Number);
  if (u.protocol !== 'https:' || u.username || u.password || u.search || u.hash || u.pathname !== '/' ||
      p.length !== 4 || p.some(n => !Number.isInteger(n) || n < 0 || n > 255) ||
      !(p[0] === 10 || p[0] === 127 || (p[0] === 192 && p[1] === 168) || (p[0] === 172 && p[1] >= 16 && p[1] <= 31))) throw Error('僅允許區域網路 HTTPS 端點');
  return u.origin;
}

// No HTTP request (including credentials) is sent until the TLS certificate pin matches.
export function pinnedAgent(pin) {
  if (!/^[a-f0-9]{64}$/.test(pin)) throw Error('無效憑證指紋');
  const agent = new https.Agent({ keepAlive: false });
  agent.createConnection = (options, callback) => {
    const socket = tls.connect({ ...options, rejectUnauthorized: false });
    let done = false;
    const finish = (error) => { if (done) return; done = true; if (error) socket.destroy(); callback(error, error ? undefined : socket); };
    socket.setTimeout(12000, () => finish(Error('連線逾時')));
    socket.once('error', finish);
    socket.once('secureConnect', () => {
      const cert = socket.getPeerCertificate();
      const actual = cert.raw && createHash('sha256').update(cert.raw).digest('hex');
      finish(actual === pin ? null : Error('PocketDrop 憑證不符，已阻擋連線'));
    });
  };
  return agent;
}

export class PocketDropClient {
  constructor(profile) { this.profile = { ...profile, endpoint: endpoint(profile.endpoint) }; }
  async request(route, method = 'GET', data, authenticated = true) {
    const agent = pinnedAgent(this.profile.certificate_sha256);
    try {
      return await new Promise((resolve, reject) => {
        const body = data === undefined ? undefined : Buffer.from(JSON.stringify(data));
        const req = https.request(this.profile.endpoint + route, { agent, method, headers: {
          ...(authenticated ? { authorization: `Bearer ${this.profile.credential}`, 'x-pocketdrop-room': this.profile.room_id } : {}),
          ...(body ? { 'content-type': 'application/json', 'content-length': body.length } : {})
        } }, res => {
          const chunks = []; let size = 0;
          res.on('data', chunk => { size += chunk.length; if (size > 2 * 1024 * 1024) res.destroy(Error('回應過大')); else chunks.push(chunk); });
          res.on('error', reject);
          res.on('end', () => {
            if (res.statusCode < 200 || res.statusCode >= 300) { const e = Error(`PocketDrop API ${res.statusCode}`); e.status = res.statusCode; reject(e); return; }
            try { resolve(size ? JSON.parse(Buffer.concat(chunks).toString('utf8')) : null); } catch { reject(Error('無效 API 回應')); }
          });
        });
        const timer = setTimeout(() => req.destroy(Error('API 逾時')), 15000);
        req.once('close', () => clearTimeout(timer)); req.on('error', reject); req.end(body);
      });
    } finally { agent.destroy(); }
  }
  async discover() {
    const bonjour = new Bonjour();
    const candidates = await new Promise(resolve => {
      const found = new Set();
      const browser = bonjour.find({ type: 'pocketdrop', protocol: 'tcp' }, service => {
        if (String(service.txt?.id) !== this.profile.device_id) return;
        for (const ip of service.addresses ?? []) { try { found.add(endpoint(`https://${ip}:${service.port}`)); } catch {} }
      });
      setTimeout(() => { browser.stop(); bonjour.destroy(); resolve([...found]); }, 3000);
    });
    const previous = this.profile.endpoint;
    for (const candidate of candidates) {
      this.profile.endpoint = candidate;
      try { const state = await this.request('/v1/state'); if (state.device_id === this.profile.device_id && state.room_id === this.profile.room_id) return state; } catch {}
    }
    this.profile.endpoint = previous;
    throw Error('PocketDrop 離線或配對已移除，請開啟 Room 電腦並使用相同 LAN');
  }
  async state() {
    try { return await this.request('/v1/state'); }
    catch (error) { if (error.status === 401 || error.status === 403) throw error; return this.discover(); }
  }
  async writeText(content) {
    if (typeof content !== 'string' || Buffer.byteLength(content, 'utf8') > 32768) throw Error('文字上限為 32768 UTF-8 bytes');
    await this.state(); // Rediscover before writing. Never retry an ambiguous write.
    await this.request('/v1/text', 'PUT', { content });
    return { shared: true };
  }
  static async pair(invite) {
    if (invite.app !== 'PocketDrop' || invite.protocol_version !== 1 || typeof invite.token !== 'string') throw Error('無效 PocketDrop 邀請');
    const client = new PocketDropClient(invite);
    const { publicKey, privateKey } = generateKeyPairSync('ed25519');
    const result = await client.request('/v1/pair', 'POST', { token: invite.token, device_id: randomUUID(), name: 'Daily-Agent', public_key: publicKey.export({ type: 'spki', format: 'der' }).toString('base64') }, false);
    if (result.room_id !== invite.room_id || result.device_id !== invite.device_id) throw Error('配對身分不符');
    return new PocketDropClient({ endpoint: invite.endpoint, certificate_sha256: invite.certificate_sha256, ...result, identity_private_key: privateKey.export({ type: 'pkcs8', format: 'der' }).toString('base64') });
  }
}
