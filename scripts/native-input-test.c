// Native X11 input into one game on the dedicated audit display. No game test hooks.
#define _DEFAULT_SOURCE
#include <X11/Xlib.h>
#include <X11/keysym.h>
#include <dirent.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static Display *display;
static Window window;
static const char *output;
static unsigned last_frame;
static bool have_frame;

static void fail(const char *message) {
    fprintf(stderr, "Native input audit: %s\n", message);
    exit(1);
}

static void path(char *buffer, const char *directory, const char *name) {
    if (snprintf(buffer, PATH_MAX, "%s/%s", directory, name) >= PATH_MAX)
        fail("capture path is too long");
}

static void focus(void) {
    XSetInputFocus(display, window, RevertToParent, CurrentTime);
    XFlush(display);
    usleep(200000);
    Window focused;
    int revert;
    XGetInputFocus(display, &focused, &revert);
    if (focused != window) fail("game did not receive input focus");
}

static void key_event(KeySym key, bool down) {
    XEvent event = {0};
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    event.xkey.time = (now.tv_sec * 1000 + now.tv_nsec / 1000000) & 0xffffffff;
    event.xkey.type = down ? KeyPress : KeyRelease;
    event.xkey.display = display;
    event.xkey.window = window;
    event.xkey.root = DefaultRootWindow(display);
    event.xkey.keycode = XKeysymToKeycode(display, key);
    event.xkey.same_screen = true;
    XSendEvent(display, window, false, down ? KeyPressMask : KeyReleaseMask, &event);
    XFlush(display);
}

static void key(KeySym key) {
    key_event(key, true);
    usleep(40000);
    key_event(key, false);
    usleep(25000);
}

static void click(int x, int y) {
    XWarpPointer(display, None, window, 0, 0, 0, 0, x, y);
    XFlush(display);
    usleep(25000);
    // Xwayland cannot reliably warp the physical Wayland pointer. Deliver motion
    // through GLFW's real callback as well, before the synthetic button callback.
    XEvent event = {0};
    event.xmotion.type = MotionNotify;
    event.xmotion.display = display;
    event.xmotion.window = window;
    event.xmotion.root = DefaultRootWindow(display);
    event.xmotion.x = x;
    event.xmotion.y = y;
    event.xmotion.same_screen = true;
    XSendEvent(display, window, false, PointerMotionMask, &event);
    event = (XEvent){0};
    event.xbutton.type = ButtonPress;
    event.xbutton.display = display;
    event.xbutton.window = window;
    event.xbutton.root = DefaultRootWindow(display);
    event.xbutton.x = x;
    event.xbutton.y = y;
    event.xbutton.button = Button1;
    event.xbutton.same_screen = true;
    XSendEvent(display, window, false, ButtonPressMask, &event);
    XFlush(display);
    usleep(40000);
    event.xbutton.type = ButtonRelease;
    XSendEvent(display, window, false, ButtonReleaseMask, &event);
    XFlush(display);
    usleep(25000);
}

// Screen targets mirror Layout's documented flat-top projection, not screenshot guesses.
// Negate q/r after flipping to target the same world tile in the rotated view.
static void tile(int q, int r) {
    XWindowAttributes size;
    XGetWindowAttributes(display, window, &size);
    double radius = fmin((size.width - 380) / 17., (size.height - 155) / 19.);
    click((size.width - 340) / 2. + 1.5 * q * radius,
          size.height / 2. + 25 + sqrt(3) * (r + q / 2.) * radius);
}

static bool complete_png(const char *filename) {
    static const unsigned char end[12] = {0, 0, 0, 0, 'I', 'E', 'N', 'D', 0xae, 0x42, 0x60, 0x82};
    unsigned char bytes[12];
    FILE *file = fopen(filename, "rb");
    if (!file) return false;
    bool complete = fseek(file, -12, SEEK_END) == 0 &&
                    fread(bytes, 1, sizeof(bytes), file) == sizeof(bytes) &&
                    memcmp(bytes, end, sizeof(bytes)) == 0;
    fclose(file);
    return complete;
}

static void copy(const char *source, const char *destination) {
    FILE *in = fopen(source, "rb");
    FILE *out = fopen(destination, "wb");
    if (!in || !out) fail("cannot copy capture");
    unsigned char buffer[8192];
    size_t count;
    while ((count = fread(buffer, 1, sizeof(buffer), in)) != 0)
        if (fwrite(buffer, 1, count, out) != count) fail("cannot write capture");
    if (ferror(in) || fclose(out) != 0) fail("capture I/O failed");
    fclose(in);
}

static void snapshot(const char *name) {
    char frames[PATH_MAX], source[PATH_MAX], destination[PATH_MAX], filename[80];
    path(frames, output, "frames");
    key(XK_F10);
    for (int attempt = 0; attempt < 100; ++attempt) {
        DIR *directory = opendir(frames);
        if (!directory) fail("capture directory missing");
        struct dirent *entry;
        unsigned newest = 0;
        bool found = false;
        while ((entry = readdir(directory)) != NULL) {
            unsigned number;
            int end = 0;
            if (sscanf(entry->d_name, "frame-%u.png%n", &number, &end) == 1 &&
                end > 0 && entry->d_name[end] == '\0' && (!found || number > newest)) {
                newest = number;
                found = true;
            }
        }
        closedir(directory);
        if (found && (!have_frame || newest > last_frame)) {
            snprintf(filename, sizeof(filename), "frame-%04u.png", newest);
            path(source, frames, filename);
            if (complete_png(source)) {
                snprintf(filename, sizeof(filename), "%s.png", name);
                path(destination, output, filename);
                copy(source, destination);
                last_frame = newest;
                have_frame = true;
                return;
            }
        }
        usleep(20000);
    }
    fail("F10 did not produce a fresh complete PNG");
}

int main(int argc, char **argv) {
    if (argc != 3 || (strcmp(argv[2], "normal") && strcmp(argv[2], "minimum")))
        fail("usage: native-input-test OUTPUT_DIRECTORY normal|minimum");
    output = argv[1];
    display = XOpenDisplay(NULL);
    if (!display) fail("cannot open dedicated audit display");
    Window root, parent, *children;
    unsigned count;
    if (!XQueryTree(display, DefaultRootWindow(display), &root, &parent, &children, &count))
        fail("cannot query audit windows");
    unsigned matches = 0;
    for (unsigned i = 0; i < count; ++i) {
        char *name = NULL;
        XFetchName(display, children[i], &name);
        if (name && !strcmp(name, "Hex Chess")) { window = children[i]; ++matches; }
        if (name) XFree(name);
    }
    XFree(children);
    if (matches != 1) fail("expected exactly one Hex Chess window on the audit display");
    if (!strcmp(argv[2], "minimum")) {
        XResizeWindow(display, window, 1000, 640);
        XFlush(display);
        usleep(300000);
    }
    focus(); snapshot("opening");
    key(XK_c); snapshot("cpu-normal");
    key(XK_c); snapshot("cpu-hard");
    XWindowAttributes initial_size;
    XGetWindowAttributes(display, window, &initial_size);
    click(initial_size.width - 320 + 120, 48); snapshot("cpu-easy");
    usleep(3800000);
    tile(0, 4); snapshot("selected"); tile(0, 3); tile(1, 3); tile(1, 2);
    tile(-3, 5); tile(-5, 5); snapshot("third-blocked");
    usleep(1300000); snapshot("released");
    tile(-5, 5); tile(-5, 4); snapshot("committed");
    key(XK_m); key(XK_Return); focus(); usleep(3800000);
    tile(0, 4); tile(0, 3); tile(1, 3); tile(1, 2);
    key(XK_space); key(XK_Down); key(XK_space); key(XK_Left);
    key(XK_space); key(XK_Down); key(XK_space); snapshot("four-plays");
    key(XK_p);
    XWarpPointer(display, None, window, 0, 0, 0, 0, 20, 20);
    XFlush(display); usleep(300000); snapshot("paused-before");
    usleep(1000000); snapshot("paused-after"); key(XK_p);
    // Both Black slots are still busy. Select an idle knight and try a legal jump.
    key(XK_Up); key(XK_Left); key(XK_space);
    key(XK_Down); key(XK_Down); key(XK_space); snapshot("black-third-blocked");
    // First pawn lands before the second: no rejected knight may fill that slot.
    usleep(350000); snapshot("black-slot-freed");
    key(XK_Right); key(XK_Up); // Restore cursor to the second pawn for flipped play.
    key(XK_h); snapshot("rules");
    XWindowAttributes size;
    XGetWindowAttributes(display, window, &size);
    click((size.width - 680) / 2 + 100, (size.height - 620) / 2 + 581);
    snapshot("movement");
    key(XK_Escape); key(XK_f); snapshot("flipped");
    // Escape cleared both selections: one Space selects Black, not two toggling it off.
    usleep(1300000); key(XK_space); key(XK_Up); key(XK_space);
    tile(0, -3); tile(0, -2); snapshot("flipped-orders");
    key_event(XK_Shift_L, true); usleep(150000); key(XK_r);
    key_event(XK_Shift_L, false); key(XK_Return); snapshot("black-resigned");
    key_event(XK_q, true); usleep(250000); XCloseDisplay(display);
    puts("Native controls complete; verify the captured HUD before claiming success.");
    return 0;
}
