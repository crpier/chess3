// Real browser touch events and rendered pixels, never game-state hooks or keyboard shortcuts.
export async function auditMobile(page, output, url) {
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.goto(url);
  await page.waitForFunction(() => document.getElementById('status').textContent === 'Tap the board to play');
  const canvas = page.locator('#canvas');
  const capture = async name => { await page.waitForTimeout(120); return canvas.screenshot({ path: `${output}/${name}.png`, scale: 'css' }); };
  const tap = async (x, y) => { const box = await canvas.boundingBox(); await page.touchscreen.tap(box.x + x, box.y + y); await page.waitForTimeout(50); };
  const geometry = async () => {
    const box = await canvas.boundingBox();
    const wide = box.width >= 500 && box.width > box.height;
    const board = wide ? { x: 8, y: 8, width: box.width - 264, height: box.height - 16 }
      : { x: 8, y: 64, width: box.width - 16, height: box.height - 176 };
    return { box, wide, board };
  };
  const point = async (q, r, flip = false, edge = false) => {
    const { board } = await geometry();
    const radius = Math.min(board.width / 17, board.height / (11 * Math.sqrt(3)));
    const sign = flip ? -1 : 1;
    return [board.x + board.width / 2 + sign * 1.5 * q * radius + (edge ? radius * 0.75 : 0),
      board.y + board.height / 2 + sign * Math.sqrt(3) * (r + q / 2) * radius];
  };
  const tile = async (...args) => tap(...await point(...args));
  const primary = async index => {
    const { box, wide } = await geometry();
    const x = wide ? box.width - 248 : 8;
    const width = wide ? 232 : box.width - 16;
    const bw = (width - 12) / 3;
    await tap(x + index * (bw + 6) + bw / 2, box.height - 32);
  };
  const menu = async index => { const { box } = await geometry(); const bw = (box.width - 60) / 3; await tap(24 + index % 3 * (bw + 6) + bw / 2, 92 + Math.floor(index / 3) * 56); };
  const footer = async (index, count) => { const { box } = await geometry(); const bw = (box.width - (count === 4 ? 66 : 54)) / count; await tap(24 + index * (bw + 6) + bw / 2, box.height - (count === 4 ? 46 : 48)); };
  await capture('unfocused');
  await tap(5, 5); await capture('opening');
  await page.waitForTimeout(3200); await capture('ready');
  const dimensions = await canvas.evaluate(element => ({ css: [element.clientWidth, element.clientHeight], backing: [element.width, element.height], density: window.devicePixelRatio }));
  if (dimensions.backing.some((value, index) => value !== Math.round(dimensions.css[index] * dimensions.density))) throw new Error('Mobile backing resolution mismatch');
  if (await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth || document.documentElement.scrollHeight > window.innerHeight)) throw new Error('Mobile page overflows the viewport');
  // Full-hex stationary selection, outside the smaller piece-art picking radius.
  await tile(0, 4, false, true); await capture('selected');
  const cdp = await page.context().newCDPSession(page);
  const box = await canvas.boundingBox();
  const [x, y] = await point(0, 3);
  const finger = { x: box.x + x, y: box.y + y, id: 1 };
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [finger] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ ...finger, x: finger.x + 40 }] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await capture('drag-ignored');
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [finger, { ...finger, x: finger.x + 40, id: 2 }] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] }); await capture('multitouch-ignored');
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [finger] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchCancel', touchPoints: [] }); await capture('cancel-ignored');
  await cdp.detach();
  await tile(0, 3); await primary(0);
  const pausedBefore = await capture('paused-before'); await page.waitForTimeout(500);
  if (!pausedBefore.equals(await capture('paused-after'))) throw new Error('Mobile manual pause changed the canvas');
  await primary(1); await capture('cpu-normal'); await primary(1); await capture('cpu-hard'); await primary(1); await capture('cpu-easy');
  await primary(0); await page.waitForTimeout(2600); await capture('bot-playing');
  await primary(2); await capture('menu'); await menu(5); await capture('rules-first');
  for (let i = 0; i < 5; i++) await footer(1, 4);
  await capture('rules-last'); await footer(2, 4); await capture('movement-king');
  for (let i = 0; i < 5; i++) await footer(1, 4);
  await capture('movement-pawn'); await footer(3, 4);
  await menu(7); await capture('promotion'); await menu(0); await menu(2); await capture('flipped');
  await primary(2); await menu(4); await capture('mode-confirm'); await footer(0, 2);
  await page.waitForTimeout(3200);
  // Queue this short public-input sequence as genuinely fast taps. Software WebGL
  // can make individual Playwright round trips slower than a pawn's travel time.
  const localGeometry = await geometry();
  const quickWidth = localGeometry.wide ? 232 : localGeometry.box.width - 16;
  const quickX = localGeometry.wide ? localGeometry.box.width - 248 : 8;
  const quickButton = (quickWidth - 12) / 3;
  const localTaps = [await point(0, 4, true), await point(0, 3, true),
    [quickX + quickButton + 6 + quickButton / 2, localGeometry.box.height - 32],
    await point(0, -4, true), await point(0, -3, true),
    [quickX + quickButton / 2, localGeometry.box.height - 32]];
  const localSession = await page.context().newCDPSession(page);
  const dispatches = [];
  for (const [x, y] of localTaps) {
    dispatches.push(localSession.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: localGeometry.box.x + x, y: localGeometry.box.y + y, id: 1 }] }));
    dispatches.push(localSession.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] }));
  }
  await Promise.all(dispatches); await page.waitForTimeout(400); await localSession.detach();
  await capture('local-orders');
  await primary(2); await menu(3); await capture('new-confirm'); await footer(1, 2); await menu(2); await capture('new-cancelled');
  // Preserve the paused orders while changing orientation, then exercise touch controls there.
  const original = page.viewportSize();
  await page.setViewportSize({ width: original.height, height: original.width });
  await page.waitForTimeout(250);
  if (original.width === 320) {
    await capture('rotation-too-small');
    await page.setViewportSize(original); await page.waitForTimeout(250);
  }
  await capture('rotated');
  await primary(2); await menu(8); await capture('resign-confirm'); await footer(0, 2); await capture('resigned');
  await primary(2); await menu(3); await footer(0, 2);
  await primary(2); await menu(4); await footer(0, 2);
  await page.waitForTimeout(3200);
  await tile(0, 4, true); await tile(0, 3, true);
  await page.locator('#fullscreen').focus();
  const blurBefore = await capture('focus-before'); await page.waitForTimeout(500);
  if (!blurBefore.equals(await capture('focus-after'))) throw new Error('Mobile focus loss changed the canvas');
  await page.locator('#fullscreen').tap(); await page.waitForTimeout(250); await capture('fullscreen');
  if (!await page.evaluate(() => document.fullscreenElement === document.body)) throw new Error('Mobile fullscreen did not start');
  await page.evaluate(() => document.exitFullscreen()); await canvas.focus();
  await page.setViewportSize({ width: 280, height: 400 }); await page.waitForTimeout(200); await capture('too-small');
  await page.setViewportSize(original); await page.waitForTimeout(200); await capture('restored');
  if (errors.length) throw new Error(errors.join('\n'));
  return { output, dimensions, errors, checks: 'touch selection, fast taps, legal move, drag/multitouch/cancel rejection, CPU reply/levels, pause, rules pages, movement pages, promotion, flip, local orders for both sides, confirmations/cancel, orientation, Black resignation, new game, focus pause, fullscreen, undersized pause, high-DPI' };
}
