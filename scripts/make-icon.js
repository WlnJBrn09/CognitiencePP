/**
 * Build multi-size Windows .ico (and icon.png) from static/assets/logo.png.
 * Multi-size ICOs are required for correct taskbar / pin icons on Windows.
 */
'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const pngToIco = require('png-to-ico');

const root = path.resolve(__dirname, '..');
const src = path.join(root, 'static', 'assets', 'logo.png');
const buildDir = path.join(root, 'build');
const outIco = path.join(buildDir, 'icon.ico');
const outPng = path.join(buildDir, 'icon.png');
const sizes = [16, 24, 32, 48, 64, 128, 256];

function resizeWithPowerShell(srcPng, outPng, size) {
  const ps = `
Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile('${srcPng.replace(/'/g, "''")}')
$bmp = New-Object System.Drawing.Bitmap ${size}, ${size}
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g.Clear([System.Drawing.Color]::Transparent)
$g.DrawImage($src, 0, 0, ${size}, ${size})
$bmp.Save('${outPng.replace(/'/g, "''")}', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose(); $src.Dispose()
`;
  execFileSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', ps],
    { stdio: 'pipe', windowsHide: true }
  );
}

async function main() {
  if (!fs.existsSync(src)) {
    console.error('Missing logo:', src);
    process.exit(1);
  }
  fs.mkdirSync(buildDir, { recursive: true });

  const tmpDir = path.join(buildDir, '_icon_sizes');
  fs.mkdirSync(tmpDir, { recursive: true });
  const pngs = [];
  for (const size of sizes) {
    const p = path.join(tmpDir, `icon-${size}.png`);
    try {
      resizeWithPowerShell(src, p, size);
      if (fs.existsSync(p) && fs.statSync(p).size > 0) pngs.push(p);
    } catch (e) {
      console.warn('resize failed for', size, e.message || e);
    }
  }

  if (!pngs.length) {
    // Fallback: single-source ICO
    const buf = await pngToIco(src);
    fs.writeFileSync(outIco, buf);
  } else {
    const buf = await pngToIco(pngs);
    fs.writeFileSync(outIco, buf);
  }

  // 256px (or largest) as icon.png for electron-builder / window icon fallback
  const best = path.join(tmpDir, 'icon-256.png');
  if (fs.existsSync(best)) fs.copyFileSync(best, outPng);
  else fs.copyFileSync(src, outPng);

  // Clean temp
  try {
    for (const p of pngs) fs.unlinkSync(p);
    fs.rmdirSync(tmpDir);
  } catch {
    /* ignore */
  }

  console.log('Wrote', outIco, '(' + fs.statSync(outIco).size + ' bytes)');
  console.log('Wrote', outPng);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
