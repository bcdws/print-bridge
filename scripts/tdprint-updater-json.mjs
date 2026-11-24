import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const NSIS_TARGET = 'windows-x86_64-nsis';

/** 由安装包、签名和发布信息构造 Tauri 更新清单。 */
export function buildUpdaterManifest({ version, notes, pubDate, signature, downloadUrl }) {
  if (!version) throw new Error('version is required');
  if (!signature) throw new Error('signature is required');
  if (!downloadUrl) throw new Error('downloadUrl is required');

  return {
    version,
    notes: notes ?? '',
    pub_date: pubDate ?? new Date().toISOString(),
    platforms: {
      [NSIS_TARGET]: {
        signature: signature.trim(),
        url: downloadUrl,
      },
    },
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [, , installerPath, signaturePath, version, downloadUrl, notesPath] = process.argv;

  if (!installerPath || !signaturePath || !version || !downloadUrl) {
    console.error(
      'Usage: node scripts/tdprint-updater-json.mjs <installer> <signature> <version> <download-url> [notes-file]',
    );
    process.exit(1);
  }

  const notes = notesPath ? readFileSync(notesPath, 'utf8') : '';
  const manifest = buildUpdaterManifest({
    version,
    notes,
    signature: readFileSync(signaturePath, 'utf8'),
    downloadUrl,
  });

  const outputPath = 'latest.json';
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`Wrote ${outputPath} for ${NSIS_TARGET} (${version}).`);
}
