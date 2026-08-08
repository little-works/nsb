import { execFileSync } from 'node:child_process';
import {
  chmod,
  copyFile,
  mkdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDirectory = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const appDirectory = resolve(rootDirectory, 'NSB.app');
const iconsetDirectory = resolve(rootDirectory, 'NSB.iconset');
const archivePath = resolve(rootDirectory, 'NSB-macos-arm64.zip');
const binaryPath = resolve(rootDirectory, 'target', 'release', 'nsb');
const iconPath = resolve(rootDirectory, 'gui', 'assets', 'nsb-logo.png');

if (process.platform !== 'darwin') {
  throw new Error('macOS app packaging can only run on darwin.');
}

function run(command, arguments_) {
  try {
    execFileSync(command, arguments_, {
      cwd: rootDirectory,
      stdio: 'inherit',
    });
  } catch (error) {
    const detail = error instanceof Error ? `: ${error.message}` : '';
    throw new Error(`Failed to run ${command}${detail}`);
  }
}

function escapeXml(value) {
  return value.replace(/[&<>"']/g, (character) => {
    switch (character) {
      case '&':
        return '&amp;';
      case '<':
        return '&lt;';
      case '>':
        return '&gt;';
      case '"':
        return '&quot;';
      case "'":
        return '&apos;';
      default:
        return character;
    }
  });
}

async function readPackageVersion() {
  const cargoToml = await readFile(
    resolve(rootDirectory, 'gui', 'Cargo.toml'),
    'utf8',
  );
  const packageHeader = cargoToml.match(/^\[package\]\s*$/m);
  const packageContents = packageHeader
    ? cargoToml.slice((packageHeader.index ?? 0) + packageHeader[0].length)
    : '';
  const nextSectionIndex = packageContents.search(/^\[/m);
  const packageSection =
    nextSectionIndex === -1
      ? packageContents
      : packageContents.slice(0, nextSectionIndex);
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];

  if (!version) {
    throw new Error('Unable to read the package version from gui/Cargo.toml.');
  }

  return version;
}

async function main() {
  const version = await readPackageVersion();
  const contentsDirectory = resolve(appDirectory, 'Contents');
  const macOsDirectory = resolve(contentsDirectory, 'MacOS');
  const resourcesDirectory = resolve(contentsDirectory, 'Resources');

  await rm(appDirectory, { force: true, recursive: true });
  await rm(iconsetDirectory, { force: true, recursive: true });
  await rm(archivePath, { force: true });
  await mkdir(macOsDirectory, { recursive: true });
  await mkdir(resourcesDirectory, { recursive: true });

  await copyFile(binaryPath, resolve(macOsDirectory, 'nsb'));
  await chmod(resolve(macOsDirectory, 'nsb'), 0o755);

  try {
    await mkdir(iconsetDirectory);
    for (const [size, name] of [
      [16, 'icon_16x16.png'],
      [32, 'icon_16x16@2x.png'],
      [32, 'icon_32x32.png'],
      [64, 'icon_32x32@2x.png'],
      [128, 'icon_128x128.png'],
      [256, 'icon_128x128@2x.png'],
      [256, 'icon_256x256.png'],
      [512, 'icon_256x256@2x.png'],
      [512, 'icon_512x512.png'],
      [1024, 'icon_512x512@2x.png'],
    ]) {
      run('sips', [
        '-z',
        String(size),
        String(size),
        iconPath,
        '--out',
        resolve(iconsetDirectory, name),
      ]);
    }

    run('iconutil', [
      '-c',
      'icns',
      iconsetDirectory,
      '-o',
      resolve(resourcesDirectory, 'nsb.icns'),
    ]);
  } finally {
    await rm(iconsetDirectory, { force: true, recursive: true });
  }

  const escapedVersion = escapeXml(version);
  await writeFile(
    resolve(contentsDirectory, 'Info.plist'),
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>nsb</string>
  <key>CFBundleIdentifier</key>
  <string>com.little-works.nsb</string>
  <key>CFBundleVersion</key>
  <string>${escapedVersion}</string>
  <key>CFBundleShortVersionString</key>
  <string>${escapedVersion}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleIconFile</key>
  <string>nsb</string>
</dict>
</plist>
`,
  );

  run('ditto', ['-c', '-k', '--keepParent', appDirectory, archivePath]);
}

await main();
