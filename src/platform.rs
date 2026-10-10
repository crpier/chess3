//! Frame scheduling only: the native and browser builds run the same game frame.
use anyhow::Result;

/// Layout and picking stay in CSS pixels; only the framebuffer uses device pixels.
#[cfg(any(target_os = "emscripten", test))]
#[derive(Clone, Copy, Debug)]
pub struct BrowserViewport {
    pub width: i32,
    pub height: i32,
    pub density: f32,
    pub touch: bool,
}

#[cfg(any(target_os = "emscripten", test))]
impl BrowserViewport {
    pub fn framebuffer(self) -> (i32, i32) {
        (
            (self.width as f32 * self.density).round().max(1.) as i32,
            (self.height as f32 * self.density).round().max(1.) as i32,
        )
    }
}

#[cfg(not(target_os = "emscripten"))]
pub fn run_frames(mut frame: impl FnMut() -> Result<bool> + 'static) -> Result<()> {
    while frame()? {}
    Ok(())
}

#[cfg(target_os = "emscripten")]
mod browser {
    use super::*;
    use std::ffi::{c_char, c_void};

    unsafe extern "C" {
        fn emscripten_set_main_loop_arg(
            callback: extern "C" fn(*mut c_void),
            arg: *mut c_void,
            fps: i32,
            simulate_infinite_loop: i32,
        );
        fn emscripten_cancel_main_loop();
        fn emscripten_run_script_int(script: *const c_char) -> i32;
        fn emscripten_run_script(script: *const c_char);
        fn emscripten_get_device_pixel_ratio() -> f64;
    }

    pub fn viewport() -> BrowserViewport {
        // Keep the CSS box independent of Emscripten's framebuffer size updates.
        unsafe {
            emscripten_run_script(c"Module.syncCanvasCssSize()".as_ptr());
            BrowserViewport {
                width: emscripten_run_script_int(
                    c"document.getElementById('stage').clientWidth".as_ptr(),
                ),
                height: emscripten_run_script_int(
                    c"document.getElementById('stage').clientHeight".as_ptr(),
                ),
                density: emscripten_get_device_pixel_ratio().clamp(1., 4.) as f32,
                touch: emscripten_run_script_int(c"Module.isTouchLayout() ? 1 : 0".as_ptr()) != 0,
            }
        }
    }

    pub fn sync_canvas_css_size() {
        unsafe { emscripten_run_script(c"Module.syncCanvasCssSize()".as_ptr()) };
    }

    pub fn take_touch_tap() -> Option<(f32, f32)> {
        unsafe {
            if emscripten_run_script_int(c"Module.touchTaps.length ? (Module.lastTouchTap = Module.touchTaps.shift(), 1) : 0".as_ptr()) == 0 {
                return None;
            }
            Some((
                emscripten_run_script_int(c"Math.round(Module.lastTouchTap.x)".as_ptr()) as f32,
                emscripten_run_script_int(c"Math.round(Module.lastTouchTap.y)".as_ptr()) as f32,
            ))
        }
    }

    pub fn take_clock_reset() -> bool {
        unsafe {
            emscripten_run_script_int(
                c"Module.resetFrameClock ? (Module.resetFrameClock = false, 1) : 0".as_ptr(),
            ) != 0
        }
    }

    pub fn focused() -> bool {
        unsafe {
            emscripten_run_script_int(
                c"!document.hidden && document.hasFocus() && document.activeElement === Module.canvas"
                    .as_ptr(),
            ) != 0
        }
    }

    pub fn run_frames<F: FnMut() -> Result<bool> + 'static>(frame: F) -> Result<()> {
        extern "C" fn tick<F: FnMut() -> Result<bool>>(arg: *mut c_void) {
            // The allocation remains alive until cancellation; only this callback accesses it.
            let frame = unsafe { &mut *arg.cast::<F>() };
            let keep_running = match frame() {
                Ok(keep_running) => keep_running,
                Err(error) => {
                    eprintln!("Game stopped: {error:#}");
                    unsafe {
                        emscripten_run_script_int(
                            c"Module.setStatus('Game stopped unexpectedly. Reload to retry.'); 0"
                                .as_ptr(),
                        );
                    }
                    false
                }
            };
            if !keep_running {
                unsafe {
                    emscripten_cancel_main_loop();
                    drop(Box::from_raw(arg.cast::<F>()));
                }
            }
        }
        let state = Box::into_raw(Box::new(frame));
        // Return from main normally; Emscripten keeps the runtime alive for animation frames.
        // No Asyncify, busy loop, or exception unwinding across Rust is required.
        unsafe { emscripten_set_main_loop_arg(tick::<F>, state.cast(), 0, 0) };
        Ok(())
    }
}

#[cfg(target_os = "emscripten")]
pub use browser::{
    focused, run_frames, sync_canvas_css_size, take_clock_reset, take_touch_tap, viewport,
};

#[cfg(test)]
mod tests {
    use super::BrowserViewport;

    #[test]
    fn browser_backing_resolution_tracks_density_without_changing_layout() {
        for (density, framebuffer) in [(1., (1000, 641)), (1.25, (1250, 801)), (2., (2000, 1282))] {
            let viewport = BrowserViewport {
                width: 1000,
                height: 641,
                density,
                touch: false,
            };
            assert_eq!(viewport.framebuffer(), framebuffer);
            assert!(!viewport.touch);
            assert_eq!((viewport.width, viewport.height), (1000, 641));
        }
    }
}
