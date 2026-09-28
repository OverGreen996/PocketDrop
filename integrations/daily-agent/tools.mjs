import { PocketDropClient } from './client.mjs';
import { loadProfile } from './profile.mjs';
const definitions = {
  pocketdrop_read_text: ['讀取 PocketDrop 共享文字（僅限使用者明確要求）', {}],
  pocketdrop_write_text: ['取代 PocketDrop 共享文字，會同步至 Room 裝置', { content: { type: 'string' } }],
  pocketdrop_list_files: ['列出 PocketDrop 檔案資訊，不下載檔案', {}]
};
export const schemas = Object.entries(definitions).map(([name, [description, properties]]) => ({ type: 'function', function: { name, description, parameters: { type: 'object', properties, required: Object.keys(properties), additionalProperties: false } } }));
export async function executePocketDrop({ tool, args }, context, clientFactory = async () => new PocketDropClient(await loadProfile())) {
  if (!Object.hasOwn(definitions, tool)) throw Error('未知 PocketDrop 工具');
  if (context?.source !== 'user' || context.deviceId || !/PocketDrop|共享暫存盒/i.test(context.userText ?? '')) throw Error('請在電腦明確要求操作 PocketDrop');
  if (!args || typeof args !== 'object' || Array.isArray(args) || Object.keys(args).some(k => !Object.hasOwn(definitions[tool][1], k))) throw Error('無效參數');
  if (tool === 'pocketdrop_write_text' && (!/寫入|貼到|分享到|放到|更新|清空|取代|write|share|replace|clear/i.test(context.userText) || typeof args.content !== 'string')) throw Error('目前訊息未授權寫入 PocketDrop');
  const client = await clientFactory();
  if (tool === 'pocketdrop_write_text') return client.writeText(args.content);
  const state = await client.state();
  if (tool === 'pocketdrop_read_text') return { content: state.text, revision: state.revision };
  return { files: state.files.map(({ file_id, name, size, origin, available, sha256 }) => ({ file_id, name, size, origin, available, sha256 })) };
}
