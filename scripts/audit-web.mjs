// Isolated headless Chromium: no connection to the user's browser or desktop focus.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { auditBrowser } from './browser-input-test.js';
import { auditMobile } from './mobile-input-test.js';

const mobileCases = [
  { name: 'phone', width: 390, height: 844, density: 3 },
  { name: 'small-phone', width: 320, height: 568, density: 2 },
  { name: 'landscape-phone', width: 844, height: 390, density: 2 },
  { name: 'tablet', width: 768, height: 1024, density: 2 },
];
if (process.env.WEB_AUDIT && !['desktop', 'mobile'].includes(process.env.WEB_AUDIT)) {
  throw new Error('WEB_AUDIT must be desktop or mobile (omit for both).');
}
if (process.env.WEB_MOBILE_CASE && !mobileCases.some(device => device.name === process.env.WEB_MOBILE_CASE)) {
  throw new Error('Unknown WEB_MOBILE_CASE; use phone, small-phone, landscape-phone or tablet.');
}

const root = resolve('target/web');
await readFile(resolve(root, 'chess3.wasm')); // Fail before launching if the build is missing.
await mkdir('artifacts/web', { recursive: true });
const output = await mkdtemp(resolve('artifacts/web/headless-'));
const files = new Map([
  ['/', ['index.html', 'text/html']],
  ['/index.html', ['index.html', 'text/html']],
  ['/host.js', ['host.js', 'text/javascript']],
  ['/chess3.js', ['chess3.js', 'text/javascript']],
  ['/chess3.wasm', ['chess3.wasm', 'application/wasm']],
]);
const server = createServer(async (request, response) => {
  const entry = files.get(new URL(request.url, 'http://localhost').pathname);
  if (!entry) { response.writeHead(404).end(); return; }
  try {
    const data = await readFile(resolve(root, entry[0]));
    response.writeHead(200, { 'Content-Type': entry[1], 'Cache-Control': 'no-store' }).end(data);
  } catch {
    response.writeHead(500).end();
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  // Override with WEB_CHROMIUM=/usr/bin/chromium to use a system installation.
  browser = await chromium.launch({ headless: true, executablePath: process.env.WEB_CHROMIUM || undefined });
  for (const density of (process.env.WEB_AUDIT === 'mobile' ? [] : [1, 2])) {
    const directory = resolve(output, `dpr-${density}`);
    await mkdir(directory);
    const context = await browser.newContext({ deviceScaleFactor: density });
    try {
      const page = await context.newPage();
      page.setDefaultTimeout(15_000);
      const result = await auditBrowser(page, directory, `http://127.0.0.1:${server.address().port}/`, density);
      execFileSync('bash', ['scripts/check-web-captures.sh', directory], { stdio: 'inherit' });
      await writeFile(resolve(directory, 'report.json'), JSON.stringify(result, null, 2));
      console.log(JSON.stringify(result, null, 2));
    } finally {
      await context.close();
    }
  }
  for (const device of (process.env.WEB_AUDIT === 'desktop' ? [] : mobileCases)) {
    if (process.env.WEB_MOBILE_CASE && process.env.WEB_MOBILE_CASE !== device.name) continue;
    const directory = resolve(output, device.name);
    await mkdir(directory);
    const context = await browser.newContext({ isMobile: true, hasTouch: true, deviceScaleFactor: device.density, viewport: { width: device.width, height: device.height } });
    try {
      const page = await context.newPage();
      page.setDefaultTimeout(15_000);
      const result = await auditMobile(page, directory, `http://127.0.0.1:${server.address().port}/`);
      execFileSync('bash', ['scripts/check-mobile-captures.sh', directory], { stdio: 'inherit' });
      await writeFile(resolve(directory, 'report.json'), JSON.stringify(result, null, 2));
      console.log(JSON.stringify(result, null, 2));
    } finally { await context.close(); }
  }

} finally {
  await browser?.close();
  await new Promise(resolve => server.close(resolve));
}
