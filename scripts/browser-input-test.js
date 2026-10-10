// Run this function against the locally served game with Playwright (scripts/audit-web.mjs).
// Only public input and rendered canvas captures; no production game-state test hooks.
export async function auditBrowser(page, output, url, density = 1) {
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.goto(url);
  await page.waitForFunction(() => document.getElementById('status').textContent === 'Click the board to play');
  const canvas = page.locator('#canvas');
  await page.waitForTimeout(200);
  const initialResolution = await canvas.evaluate(element => ({
    backing: [element.width, element.height],
    css: [element.clientWidth, element.clientHeight],
    density: window.devicePixelRatio,
  }));
  if (initialResolution.backing.some((value, index) => value !== Math.round(initialResolution.css[index] * density))) {
    throw new Error(`Canvas is not rendering at device resolution: ${JSON.stringify(initialResolution)}`);
  }
  const capture = async name => {
    await page.waitForTimeout(100);
    return canvas.screenshot({ path: `${output}/${name}.png`, scale: 'css' });
  };
  const key = async name => { await page.keyboard.press(name, { delay: 30 }); await page.waitForTimeout(30); };
  const tile = async (q, r) => {
    const box = await canvas.boundingBox();
    const radius = Math.max(8, Math.min((box.width - 380) / 17, (box.height - 155) / 19));
    await page.mouse.click(box.x + (box.width - 340) / 2 + 1.5 * q * radius,
      box.y + box.height / 2 + 25 + Math.sqrt(3) * (r + q / 2) * radius, { delay: 30 });
    await page.waitForTimeout(30);
  };
  await capture('unfocused');
  // Focus without selecting, then let the opening countdown finish.
  await canvas.click({ position: { x: 20, y: 150 } });
  await capture('opening');
  await page.waitForTimeout(3200);
  await tile(0, 4);
  await capture('selected');
  await canvas.screenshot({ path: `${output}/selected-device.png`, scale: 'device' });
  await tile(0, 3);
  await key('p');
  const before = await capture('paused-before');
  await page.waitForTimeout(500);
  const after = await capture('paused-after');
  if (!before.equals(after)) throw new Error('Manual pause changed the rendered canvas');
  await key('p');
  await page.waitForTimeout(2600);
  await capture('bot-playing');
  await key('c'); await capture('cpu-normal');
  await key('c'); await capture('cpu-hard');
  await key('c'); await capture('cpu-easy');
  await key('h'); await capture('rules'); await key('Escape');
  // Restart into local mode; both sides get their normal controls.
  await key('m'); await key('Enter');
  await page.waitForTimeout(3200);
  await tile(0, 4); await tile(0, 3);
  await key('Space'); await key('ArrowDown'); await key('Space');
  await key('p'); await capture('local-orders');
  // A real Tab key must escape GLFW's otherwise window-wide keyboard trap.
  await key('Tab');
  if (await canvas.evaluate(element => document.activeElement === element)) throw new Error('Tab trapped canvas focus');
  // Pause manually remains set until refocus; Q/C typed off-canvas must not change the game.
  const focusedBefore = await capture('blur-before');
  await key('c'); await key('q');
  await page.waitForTimeout(400);
  const focusedAfter = await capture('blur-after');
  if (!focusedBefore.equals(focusedAfter)) throw new Error('Off-canvas keys changed the game');
  await canvas.focus(); await key('p');
  // Tab during running play freezes both active orders, not merely manual-pause state.
  await key('Tab');
  const lostBefore = await capture('focus-before');
  await page.waitForTimeout(500);
  const lostAfter = await capture('focus-after');
  if (!lostBefore.equals(lostAfter)) throw new Error('Focus loss changed the rendered canvas');
  await canvas.focus(); await key('h'); await key('Escape');
  await page.setViewportSize({ width: 1000, height: 723 });
  await page.waitForTimeout(300); await capture('minimum');
  const dimensions = await canvas.evaluate(element => [element.width, element.height, element.clientWidth, element.clientHeight]);
  if (dimensions[2] !== 1000 || dimensions[3] !== 640 || dimensions[0] !== Math.round(dimensions[2] * density) || dimensions[1] !== Math.round(dimensions[3] * density)) {
    throw new Error(`Canvas resize mismatch: ${dimensions}`);
  }
  await page.setViewportSize({ width: 900, height: 600 });
  await page.waitForTimeout(300); await capture('too-small');
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.waitForTimeout(200);
  await key('q'); await capture('q-paused'); await key('p');
  // Fullscreen requires real user input rather than an untrusted DOM click.
  await page.locator('#fullscreen').click();
  await page.waitForTimeout(400);
  if (!await page.evaluate(() => document.fullscreenElement === document.body)) throw new Error('Fullscreen did not start');
  await capture('fullscreen');
  await page.evaluate(() => document.exitFullscreen());
  await page.waitForTimeout(200); await canvas.focus();
  await key('p'); await capture('finished');
  // Moving between differently scaled displays / changing browser zoom must update
  // the backing resolution even if the CSS viewport dimensions are unchanged.
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: 1280, height: 1000, deviceScaleFactor: 1.25, mobile: false,
  });
  await page.waitForFunction(() => {
    const canvas = document.getElementById('canvas');
    return canvas.width === Math.round(canvas.clientWidth * 1.25)
      && canvas.height === Math.round(canvas.clientHeight * 1.25);
  });
  const changedResolution = await canvas.evaluate(element => [element.width, element.height, element.clientWidth, element.clientHeight]);
  await canvas.focus(); await key('n'); await key('Enter');
  await page.waitForTimeout(3200);
  await tile(0, 4); await capture('density-changed-selected');
  await tile(0, 3); await key('p'); await capture('density-changed-move');
  await cdp.detach();
  if (errors.length) throw new Error(errors.join('\n'));
  return { output, initialResolution, dimensions, changedResolution, errors, checks: 'load, selection, move, pause, CPU reply, CPU levels, rules, local controls, Tab, off-canvas keys, focus pause, resize, Q, fullscreen, live fractional DPI change' };
}
