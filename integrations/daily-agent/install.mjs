// Explicit local installation; fail closed if the upstream ToolBroker structure changes.
import { readFile, writeFile, copyFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
const root = process.argv[2];
if (!root) throw Error('用法：node install.mjs <Daily-Agent/daily-agent 資料夾>');
const file = join(resolve(root), 'tools', 'ToolBroker.js');
let source = (await readFile(file, 'utf8')).replace(/\r\n/g, '\n');
if (source.includes('executePocketDrop')) { console.log('PocketDrop 介接已安裝'); process.exit(0); }
const edits = [
  ['return Object.entries(definitions).map', 'return [...pocketDropSchemas, ...Object.entries(definitions).map'],
  ['    }));\n  }\n  async execute', '    }))];\n  }\n  async execute'],
  ['    this.permissions.authorize(tool, args, context);', '    if (tool.startsWith("pocketdrop_")) return executePocketDrop({ tool, args }, context);\n    this.permissions.authorize(tool, args, context);']
];
for (const [before, after] of edits) {
  if (source.split(before).length !== 2) throw Error('Daily-Agent 工具介面已改變，未修改檔案；請依 README 手動介接');
  source = source.replace(before, after);
}
const moduleURL = pathToFileURL(fileURLToPath(new URL('./tools.mjs', import.meta.url))).href;
source = `import { schemas as pocketDropSchemas, executePocketDrop } from ${JSON.stringify(moduleURL)};\n` + source;
await copyFile(file, file + '.before-pocketdrop', 1);
await writeFile(file, source, 'utf8');
console.log('已加入 PocketDrop 工具。請保留介接模組所在資料夾，重新啟動 Daily-Agent。');
