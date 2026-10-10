'use strict';
const canvas = document.getElementById('canvas');
const status = document.getElementById('status');
const readyMessage = () => Module.isTouchLayout() ? 'Tap the board to play' : 'Click the board to play';
var Module = {
  canvas,
  resetFrameClock: true,
  touchTaps: [],
  isTouchLayout: () => window.matchMedia('(pointer: coarse)').matches,
  syncCanvasCssSize() {
    const stage = document.getElementById('stage');
    canvas.style.width = `${stage.clientWidth}px`;
    canvas.style.height = `${stage.clientHeight}px`;
  },
  setStatus(message) { status.textContent = message || readyMessage(); },
  print: (...args) => console.log(...args),
  printErr: (...args) => console.error(...args),
  onRuntimeInitialized() { this.setStatus(''); },
  onAbort() { this.setStatus('Game could not start. Check WebGL support or reload to retry.'); }
};
// GLFW installs window-wide capture listeners. Keep Tab and browser shortcuts out
// of those listeners, and do not send game commands while a page control is focused.
for (const type of ['keydown', 'keypress']) {
  window.addEventListener(type, event => {
    if (document.activeElement !== canvas || event.key === 'Tab' || event.ctrlKey || event.metaKey) {
      event.stopImmediatePropagation();
    }
  }, true);
}
// Visibility changes can suspend animation callbacks before Rust observes a lost focus.
// Remember the interruption so the first resumed frame discards all elapsed wall time.
const resetClock = () => { Module.resetFrameClock = true; };
window.addEventListener('blur', resetClock);
document.addEventListener('visibilitychange', resetClock);
canvas.addEventListener('blur', resetClock);
canvas.addEventListener('pointerdown', () => canvas.focus({ preventScroll: true }));
// GLFW's touch-as-mouse press can lose a fast tap between animation frames,
// and submits on finger-down. Own touch input instead: one completed, stationary,
// single-finger tap becomes one CSS-coordinate input. Mouse input stays untouched.
let touchStart = null;
for (const type of ['touchstart', 'touchmove', 'touchend', 'touchcancel']) {
  canvas.addEventListener(type, event => {
    if (!Module.isTouchLayout()) return;
    if (event.cancelable) event.preventDefault();
    event.stopImmediatePropagation();
    if (type === 'touchstart') {
      canvas.focus({ preventScroll: true });
      const touch = event.touches[0];
      touchStart = event.touches.length === 1 ? { id: touch.identifier, x: touch.clientX, y: touch.clientY } : null;
    } else if (type === 'touchcancel') {
      touchStart = null;
    } else if (touchStart) {
      const touch = Array.from(event.changedTouches).find(touch => touch.identifier === touchStart.id);
      if (!touch) return;
      if (Math.hypot(touch.clientX - touchStart.x, touch.clientY - touchStart.y) > 12) touchStart = null;
      if (type === 'touchend' && touchStart) {
        const rect = canvas.getBoundingClientRect();
        if (event.touches.length === 0 && Module.touchTaps.length < 16) {
          Module.touchTaps.push({ x: (touch.clientX - rect.left) * canvas.clientWidth / rect.width, y: (touch.clientY - rect.top) * canvas.clientHeight / rect.height });
        }
        touchStart = null;
      }
    }
  }, { capture: true, passive: false });
}
const clearTouchInput = () => { touchStart = null; Module.touchTaps.length = 0; };
window.addEventListener('blur', clearTouchInput);
document.addEventListener('visibilitychange', clearTouchInput);
window.addEventListener('resize', clearTouchInput);
canvas.addEventListener('blur', clearTouchInput);
canvas.addEventListener('contextmenu', event => event.preventDefault());
// Keep local-play controls from scrolling the page. Tab remains available to leave the game.
canvas.addEventListener('keydown', event => {
  if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', ' ', 'Backspace'].includes(event.key)) {
    event.preventDefault();
  }
});
document.getElementById('fullscreen').addEventListener('click', async () => {
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.body.requestFullscreen();
    canvas.focus({ preventScroll: true });
  } catch {
    Module.setStatus('Fullscreen unavailable. You can still play in this window.');
  }
});
canvas.addEventListener('webglcontextlost', event => {
  event.preventDefault();
  Module.setStatus('Graphics context lost. Reload to resume playing.');
});
