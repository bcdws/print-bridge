import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('.', import.meta.url)), '..');

const VERSION_FILES = [
  'apps/desktop/package.json',
  'apps/desktop/src-tauri/tauri.conf.json',
  'Cargo.toml',
];

/** 校验版本号是否为合法的 SemVer（可带预发布标识）。 */
export function validateVersion(version) {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Expected a SemVer version, received: ${version}`);
  }
  return version;
}

/** 比较两个 SemVer 版本，返回 -1/0/1。 */
export function compareVersions(left, right) {
  const parse = (value) => {
    const [core, prerelease] = value.split('-', 2);
    return { core: core.split('.').map(Number), prerelease: prerelease ?? null };
  };
  const a = parse(left);
  const b = parse(right);

  for (let index = 0; index < 3; index += 1) {
    if (a.core[index] !== b.core[index]) return a.core[index] > b.core[index] ? 1 : -1;
  }

  if (a.prerelease === b.prerelease) return 0;
  if (a.prerelease === null) return 1;
  if (b.prerelease === null) return -1;
  return a.prerelease > b.prerelease ? 1 : -1;
}

/** 计算下一个版本号；不带参数时递增 patch 段。 */
export function nextVersion(current, bump = 'patch') {
  const [core] = current.split('-', 2);
  const [major, minor, patch] = core.split('.').map(Number);

  switch (bump) {
    case 'patch':
      return `${major}.${minor}.${patch + 1}`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'major':
      return `${major + 1}.0.0`;
    default:
      return validateVersion(bump);
  }
}

/** 读取三个版本来源，并确保它们一致。 */
export function readVersions(repositoryRoot = root) {
  const packageJson = JSON.parse(
    readFileSync(resolve(repositoryRoot, 'apps/desktop/package.json'), 'utf8'),
  ).version;
  const tauriConf = JSON.parse(
    readFileSync(resolve(repositoryRoot, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8'),
  ).version;
  const cargoToml = readFileSync(resolve(repositoryRoot, 'Cargo.toml'), 'utf8').match(
    /^version = "([^"]+)"/m,
  )?.[1];

  const versions = { packageJson, tauriConf, cargoToml };
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1 || !cargoToml) {
    throw new Error(
      `Version fields differ:\n` +
        `  apps/desktop/package.json: ${packageJson}\n` +
        `  apps/desktop/src-tauri/tauri.conf.json: ${tauriConf}\n` +
        `  Cargo.toml workspace package: ${cargoToml}`,
    );
  }

  return { version: packageJson, files: versions };
}

/** 把版本号写入三个来源，并同步 Cargo.lock 中的 workspace 包版本。 */
export function writeVersion(version, repositoryRoot = root) {
  validateVersion(version);

  const packagePath = resolve(repositoryRoot, 'apps/desktop/package.json');
  const packageJson = JSON.parse(readFileSync(packagePath, 'utf8'));
  packageJson.version = version;
  writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

  const tauriPath = resolve(repositoryRoot, 'apps/desktop/src-tauri/tauri.conf.json');
  const tauriConf = JSON.parse(readFileSync(tauriPath, 'utf8'));
  tauriConf.version = version;
  writeFileSync(tauriPath, `${JSON.stringify(tauriConf, null, 2)}\n`);

  const cargoPath = resolve(repositoryRoot, 'Cargo.toml');
  const cargoToml = readFileSync(cargoPath, 'utf8').replace(
    /^version = "[^"]+"/m,
    `version = "${version}"`,
  );
  writeFileSync(cargoPath, cargoToml);

  const lockPath = resolve(repositoryRoot, 'Cargo.lock');
  const lock = readFileSync(lockPath, 'utf8');
  const nextLock = lock.replace(
    /(\[\[package\]\]\nname = "print-bridge-(?:cli|core|desktop|runtime|server)"\nversion = ")[^"]+(")/g,
    `$1${version}$2`,
  );
  writeFileSync(lockPath, nextLock);

  return version;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [, , mode, value] = process.argv;

  if (mode === 'current') {
    console.log(readVersions().version);
  } else if (mode === 'next') {
    console.log(nextVersion(readVersions().version, value ?? 'patch'));
  } else if (mode === 'write') {
    console.log(writeVersion(validateVersion(value ?? '')));
  } else {
    console.error('Usage: node scripts/tdprint-version.mjs <current|next|write> [bump|version]');
    process.exit(1);
  }
}
