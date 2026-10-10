import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

function host() {
  const handlers = new Map();
  const register = (id, type, callback) => {
    const key = `${id}:${type}`;
    const previous = handlers.get(key);
    handlers.set(key, previous ? event => { previous(event); callback(event); } : callback);
  };
  const element = id => ({
    id,
    textContent: '',
    style: {},
    clientWidth: 390, clientHeight: 788,
    getBoundingClientRect() { return { left: 0, top: 56, width: this.clientWidth, height: this.clientHeight }; },
    addEventListener(type, callback) { register(id, type, callback); },
    focus() { document.activeElement = this; },
  });
  const canvas = element('canvas');
  const status = element('status');
  const fullscreen = element('fullscreen');
  const window = element('window');
  window.matchMedia = () => ({ matches: false });
  const stage = { clientWidth: 1000, clientHeight: 640 };
  const document = {
    addEventListener(type, callback) { register('document', type, callback); },
    activeElement: fullscreen,
    fullscreenElement: null,
    getElementById(id) { return { canvas, status, fullscreen, stage }[id]; },
    body: { async requestFullscreen() { document.fullscreenElement = this; } },
    async exitFullscreen() { document.fullscreenElement = null; },
  };
  const context = vm.createContext({ document, window, console });
  vm.runInContext(readFileSync(new URL('./host.js', import.meta.url), 'utf8'), context);
  return { handlers, canvas, status, stage, document, window, module: context.Module };
}
function event(key, extras = {}) {
  return {
    key, stopped: false, prevented: false, cancelable: true,
    stopImmediatePropagation() { this.stopped = true; },
    preventDefault() { this.prevented = true; },
    ...extras,
  };
}

test('loading and abort messages are visible without exposing game state', () => {
  const { module, status } = host();
  module.setStatus('Downloading…'); assert.equal(status.textContent, 'Downloading…');
  module.onRuntimeInitialized(); assert.equal(status.textContent, 'Click the board to play');
  module.onAbort(); assert.match(status.textContent, /could not start/);
});
test('pointer focuses the canvas, but Tab and browser shortcuts escape GLFW', () => {
  const { handlers, canvas, document } = host();
  const outside = event('c'); handlers.get('window:keydown')(outside); assert.equal(outside.stopped, true);
  handlers.get('canvas:pointerdown')(); assert.equal(document.activeElement, canvas);
  const inside = event('c'); handlers.get('window:keydown')(inside); assert.equal(inside.stopped, false);
  for (const input of [event('Tab'), event('r', { ctrlKey: true }), event('q', { metaKey: true })]) {
    handlers.get('window:keydown')(input); assert.equal(input.stopped, true); assert.equal(input.prevented, false);
  }
});
test('local movement keys prevent page scrolling, not ordinary keys', () => {
  const { handlers } = host();
  for (const key of ['ArrowDown', 'ArrowLeft', 'ArrowRight', 'ArrowUp', ' ', 'Backspace']) {
    const input = event(key); handlers.get('canvas:keydown')(input); assert.equal(input.prevented, true);
  }
  for (const key of ['Tab', 'c']) {
    const input = event(key); handlers.get('canvas:keydown')(input); assert.equal(input.prevented, false);
  }
});
test('fullscreen toggles and restores game focus', async () => {
  const { handlers, document, canvas } = host();
  await handlers.get('fullscreen:click')();
  assert.equal(document.fullscreenElement, document.body); assert.equal(document.activeElement, canvas);
  await handlers.get('fullscreen:click')(); assert.equal(document.fullscreenElement, null);
});
test('fullscreen rejection and context loss produce actionable errors', async () => {
  const { handlers, document, status } = host();
  document.body.requestFullscreen = async () => { throw new Error('Denied'); };
  await handlers.get('fullscreen:click')(); assert.match(status.textContent, /Fullscreen unavailable/);
  const lost = event(); handlers.get('canvas:webglcontextlost')(lost);
  assert.equal(lost.prevented, true); assert.match(status.textContent, /Graphics context lost/);
});

test('focus and visibility interruptions reset the resumed frame clock', () => {
  const { handlers, module } = host();
  for (const type of ['window:blur', 'canvas:blur', 'document:visibilitychange']) {
    module.resetFrameClock = false;
    handlers.get(type)();
    assert.equal(module.resetFrameClock, true);
  }
});

test('canvas CSS dimensions remain logical pixels across backing resize', () => {
  const { module, canvas, stage } = host();
  canvas.width = 2000; canvas.height = 1280;
  module.syncCanvasCssSize();
  assert.equal(canvas.style.width, '1000px'); assert.equal(canvas.style.height, '640px');
  assert.equal(canvas.width, 2000); assert.equal(canvas.height, 1280);
  stage.clientWidth = 1200; stage.clientHeight = 800;
  module.syncCanvasCssSize();
  assert.equal(canvas.style.width, '1200px'); assert.equal(canvas.style.height, '800px');
});

function touchEvent(touches, changedTouches = touches) { return event('', { touches, changedTouches }); }
const finger = (x, y, identifier = 1) => ({ identifier, clientX: x, clientY: y });

test('completed single-finger taps are delivered once in CSS coordinates, including fast taps', () => {
  const { window, handlers, module } = host(); window.matchMedia = () => ({ matches: true });
  const start = touchEvent([finger(100, 156)]); handlers.get('canvas:touchstart')(start);
  assert.equal(start.prevented, true); assert.equal(start.stopped, true); assert.equal(module.touchTaps.length, 0);
  handlers.get('canvas:touchend')(touchEvent([], [finger(100, 156)]));
  assert.equal(module.touchTaps.length, 1); assert.equal(module.touchTaps[0].x, 100); assert.equal(module.touchTaps[0].y, 100);
  handlers.get('canvas:touchend')(touchEvent([], [finger(100, 156)])); assert.equal(module.touchTaps.length, 1);
});

test('drags, multi-touch and cancellations never become move commands', () => {
  const { window, handlers, module } = host(); window.matchMedia = () => ({ matches: true });
  const send = (type, touches, changed = touches) => handlers.get(`canvas:${type}`)(touchEvent(touches, changed));
  send('touchstart', [finger(100, 156)]); send('touchmove', [finger(130, 156)]); send('touchend', [], [finger(100, 156)]);
  assert.equal(module.touchTaps.length, 0);
  send('touchstart', [finger(100, 156)]); send('touchstart', [finger(100, 156), finger(200, 156, 2)], [finger(200, 156, 2)]);
  send('touchend', [finger(200, 156, 2)], [finger(100, 156)]); send('touchend', [], [finger(200, 156, 2)]);
  assert.equal(module.touchTaps.length, 0);
  send('touchstart', [finger(100, 156)]); send('touchcancel', [], [finger(100, 156)]); send('touchend', [], [finger(100, 156)]);
  assert.equal(module.touchTaps.length, 0);
});

test('touch input is bounded and discarded on interruptions; desktop events stay with GLFW', () => {
  const { window, handlers, module } = host();
  const mouseLayoutTouch = touchEvent([finger(100, 156)]); handlers.get('canvas:touchstart')(mouseLayoutTouch);
  assert.equal(mouseLayoutTouch.prevented, false); assert.equal(mouseLayoutTouch.stopped, false);
  window.matchMedia = () => ({ matches: true });
  for (let i = 0; i < 20; i++) {
    handlers.get('canvas:touchstart')(touchEvent([finger(100, 156)]));
    handlers.get('canvas:touchend')(touchEvent([], [finger(100, 156)]));
  }
  assert.equal(module.touchTaps.length, 16);
  for (const type of ['window:blur', 'document:visibilitychange', 'window:resize', 'canvas:blur']) {
    module.touchTaps.push({ x: 10, y: 10 }); handlers.get(type)(); assert.equal(module.touchTaps.length, 0);
  }
  module.onRuntimeInitialized(); assert.equal(module.canvas.id, 'canvas');
});
