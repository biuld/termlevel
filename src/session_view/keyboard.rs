//! AppKit events provide physical key identity, repeats and releases which the
//! NSH shortcut API does not carry. Only our exact field editor is intercepted.
use crate::sessions::Session;
use nsh_core::prelude::*;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSAccessibility, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSTextField,
    NSTextInputClient, NSTextView, NSView, NSWindow,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ptr::NonNull,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
use termlevel_ghostty_vt::{KeyEvent, raw::*};
/// NSH's scripted driver bypasses local monitors. Only test runs opt into
/// shortcut fallback; production has a single owner for physical key events.
pub(super) fn synthetic_keys() -> bool {
    #[cfg(debug_assertions)]
    {
        synthetic_flag().load(Ordering::Acquire)
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}
#[cfg(debug_assertions)]
fn synthetic_flag() -> &'static AtomicBool {
    static VALUE: std::sync::OnceLock<AtomicBool> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| AtomicBool::new(std::env::var_os("TERMLEVEL_SYNTHETIC_INPUT").is_some()))
}
#[cfg(debug_assertions)]
pub(super) fn set_synthetic_keys(value: bool) {
    synthetic_flag().store(value, Ordering::Release);
}
/// Debug tests post character keys that must reach Ghostty even when the host
/// input source is an IME. Production leaves those keys with AppKit.
pub(super) fn physical_keys() -> bool {
    #[cfg(debug_assertions)]
    {
        physical_flag().load(Ordering::Acquire)
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}
#[cfg(debug_assertions)]
fn physical_flag() -> &'static AtomicBool {
    static VALUE: std::sync::OnceLock<AtomicBool> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| AtomicBool::new(false))
}
#[cfg(debug_assertions)]
pub(super) fn set_physical_keys(value: bool) {
    physical_flag().store(value, Ordering::Release);
}
struct Monitor(Retained<objc2::runtime::AnyObject>);
impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe { NSEvent::removeMonitor(&self.0) }
    }
}
thread_local! {static MONITORS:RefCell<Vec<(Arc<AtomicBool>,Monitor)>>=const{RefCell::new(Vec::new())};}

pub(super) fn install(
    session: Arc<Session>,
    active: Prop<bool>,
    cw: Memo<Points>,
    lh: Prop<Points>,
) {
    let enabled = Arc::new(AtomicBool::new(false));
    let gate = enabled.clone();
    // Stay armed while this pane is the active session. A layout pass can
    // blur the field editor; requiring its focus signal dropped every key.
    effect(move || gate.store(active.get(), Ordering::Release));
    let gate = enabled.clone();
    cleanup(move || {
        gate.store(false, Ordering::Release);
        let gate = gate.clone();
        drop(run_on_main_thread(move || {
            MONITORS.with(|m| m.borrow_mut().retain(|(g, _)| !Arc::ptr_eq(g, &gate)));
        }));
    });
    let metrics = Arc::new(Mutex::new((1.0_f32, 1.0_f32)));
    let dimensions = metrics.clone();
    effect(move || *dimensions.lock().unwrap() = (cw.get().0, lh.get().0));
    let grid_id = format!("terminal-grid-{}", session.info().id);
    let weak = Arc::downgrade(&session);
    let id = format!("terminal-input-{}", session.info().id);
    drop(run_on_main_thread(move || {
        let gate = enabled.clone();
        let held = RefCell::new(HashMap::new());
        let callback = block2::RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit invokes local monitors on its main thread with a live event.
            let e = unsafe { event.as_ref() };
            if !gate.load(Ordering::Acquire) {
                held.borrow_mut().clear();
                return event.as_ptr();
            }
            let consumed = if matches!(
                e.r#type(),
                NSEventType::OtherMouseDown | NSEventType::OtherMouseUp
            ) {
                process_mouse(e, &weak, &grid_id, &metrics)
            } else {
                process(e, &weak, &id, &held)
            };
            if consumed {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
        let mask = NSEventMask::KeyDown
            | NSEventMask::KeyUp
            | NSEventMask::FlagsChanged
            | NSEventMask::OtherMouseDown
            | NSEventMask::OtherMouseUp;
        if let Some(m) =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &callback) }
        {
            MONITORS.with(|v| v.borrow_mut().push((enabled, Monitor(m))));
        }
    }));
}
fn find_view(root: &NSView, id: &str) -> Option<Retained<NSView>> {
    if root
        .accessibilityIdentifier()
        .is_some_and(|s| s.to_string() == id)
    {
        return unsafe { Retained::retain(root as *const NSView as *mut NSView) };
    }
    let children = root.subviews();
    (0..children.count()).find_map(|i| find_view(&children.objectAtIndex(i), id))
}
fn process_mouse(e: &NSEvent, weak: &Weak<Session>, id: &str, metrics: &Mutex<(f32, f32)>) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(window) = e.window(mtm) else {
        return false;
    };
    let Some(root) = window.contentView() else {
        return false;
    };
    let Some(grid) = find_view(&root, id) else {
        return false;
    };
    let p = grid.convertPoint_fromView(e.locationInWindow(), None);
    let bounds = grid.bounds();
    let visible = grid.visibleRect();
    let x = (p.x - bounds.origin.x) as f32;
    let mut y = (p.y - bounds.origin.y) as f32;
    if !grid.isFlipped() {
        y = bounds.size.height as f32 - y;
    }
    if x < 0.0 || y < 0.0 || x >= bounds.size.width as f32 || y >= bounds.size.height as f32 {
        return false;
    }
    if p.x < visible.origin.x
        || p.y < visible.origin.y
        || p.x >= visible.origin.x + visible.size.width
        || p.y >= visible.origin.y + visible.size.height
    {
        return false;
    }
    let flags = e.modifierFlags();
    if flags.contains(NSEventModifierFlags::Shift) {
        return false;
    }
    let Some(session) = weak.upgrade() else {
        return false;
    };
    if !session.emulator.lock().unwrap().mouse_enabled() {
        return false;
    }
    let button = match e.buttonNumber() {
        2 => GHOSTTY_MOUSE_BUTTON_MIDDLE,
        3 => GHOSTTY_MOUSE_BUTTON_EIGHT,
        4 => GHOSTTY_MOUSE_BUTTON_NINE,
        _ => return false,
    };
    let (cw, lh) = *metrics.lock().unwrap();
    let w = cw.round().max(1.0);
    let h = lh.round().max(1.0);
    let pressed = e.r#type() == NSEventType::OtherMouseDown;
    session.mouse_input(
        termlevel_ghostty_vt::MouseEvent {
            action: if pressed {
                GHOSTTY_MOUSE_ACTION_PRESS
            } else {
                GHOSTTY_MOUSE_ACTION_RELEASE
            },
            button: Some(button),
            modifiers: mods(flags),
            x: x / cw.max(1.0) * w,
            y: y / lh.max(1.0) * h,
            any_button_pressed: pressed,
        },
        w as u32,
        h as u32,
    );
    true
}
fn editor_matches(view: &NSView, id: &str, editor: &NSTextView) -> bool {
    if view
        .accessibilityIdentifier()
        .is_some_and(|s| s.to_string() == id)
    {
        if let Some(field) = view.downcast_ref::<NSTextField>() {
            return field.currentEditor().is_some_and(|e| {
                std::ptr::eq(
                    &*e as *const _ as *const (),
                    editor as *const _ as *const (),
                )
            });
        }
        let children = view.subviews();
        return (0..children.count()).any(|i| editor_in(&children.objectAtIndex(i), editor));
    }
    let children = view.subviews();
    (0..children.count()).any(|i| editor_matches(&children.objectAtIndex(i), id, editor))
}
fn editor_in(view: &NSView, editor: &NSTextView) -> bool {
    if let Some(field) = view.downcast_ref::<NSTextField>()
        && field.currentEditor().is_some_and(|e| {
            std::ptr::eq(
                &*e as *const _ as *const (),
                editor as *const _ as *const (),
            )
        })
    {
        return true;
    }
    let children = view.subviews();
    (0..children.count()).any(|i| editor_in(&children.objectAtIndex(i), editor))
}
enum KeyOwner {
    /// Our field editor. IME composition stays with it.
    Ours,
    /// Another text field (find, rename, settings). Do not take the key.
    Foreign,
    /// Click landed in this terminal, but the field editor is not focused.
    Grid,
    /// The window itself has no focused control.
    Bare,
    Other,
}
fn key_owner(window: &NSWindow, input_id: &str) -> KeyOwner {
    let Some(root) = window.contentView() else {
        return KeyOwner::Other;
    };
    let Some(responder) = window.firstResponder() else {
        return KeyOwner::Bare;
    };
    if let Some(editor) = responder.downcast_ref::<NSTextView>() {
        return if editor_matches(&root, input_id, editor) {
            KeyOwner::Ours
        } else {
            KeyOwner::Foreign
        };
    }
    let grid_id = input_id.replacen("terminal-input-", "terminal-grid-", 1);
    if let Some(view) = responder.downcast_ref::<NSView>() {
        let inside = find_view(&root, &grid_id).is_some_and(|grid| {
            std::ptr::eq(view as *const NSView, &*grid as *const NSView)
                || view.isDescendantOf(&grid)
        });
        return if inside {
            KeyOwner::Grid
        } else {
            KeyOwner::Other
        };
    }
    KeyOwner::Bare
}
fn claim_field(window: &NSWindow, input_id: &str) -> bool {
    let Some(root) = window.contentView() else {
        return false;
    };
    let Some(view) = find_view(&root, input_id) else {
        return false;
    };
    let target = text_target(&view).unwrap_or(view);
    window.makeFirstResponder(Some(&*target))
}
fn text_target(view: &NSView) -> Option<Retained<NSView>> {
    if view.downcast_ref::<NSTextField>().is_some() || view.downcast_ref::<NSTextView>().is_some() {
        return unsafe { Retained::retain(view as *const NSView as *mut NSView) };
    }
    let children = view.subviews();
    (0..children.count()).find_map(|i| text_target(&children.objectAtIndex(i)))
}
fn process(
    e: &NSEvent,
    weak: &Weak<Session>,
    id: &str,
    held: &RefCell<HashMap<u16, Option<char>>>,
) -> bool {
    if synthetic_keys() {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(window) = e.window(mtm) else {
        return false;
    };
    let owner = key_owner(&window, id);
    // Find, rename and settings keep their own text fields. Every other
    // focus owner loses keys while this session is active: the root view
    // sits outside the grid, so a failed input.focus() used to drop typing.
    if matches!(owner, KeyOwner::Foreign) {
        return false;
    }
    let ours = matches!(owner, KeyOwner::Ours);
    let Some(session) = weak.upgrade() else {
        return false;
    };
    let code = e.keyCode();
    let flags = e.modifierFlags();
    let key = physical_key(code);
    let up = e.r#type() == NSEventType::KeyUp;
    let changed = e.r#type() == NSEventType::FlagsChanged;
    if !up && !changed && reserved(code, flags) {
        return false;
    }
    if ours
        && window
            .firstResponder()
            .and_then(|r| r.downcast::<NSTextView>().ok())
            .is_some_and(|editor| editor.hasMarkedText())
    {
        held.borrow_mut().clear();
        return false;
    }
    // A focused field editor owns IME composition. Character keys on a
    // non-ASCII source stay with it. If focus is elsewhere, give the field
    // the key so the same insertion path runs; encode when it cannot focus.
    if !changed
        && !up
        && character_key(code)
        && defer_character_to_ime(code, flags)
        && (ours || claim_field(&window, id))
    {
        return false;
    }
    let action = if changed {
        let mask = match code {
            56 | 60 => NSEventModifierFlags::Shift,
            59 | 62 => NSEventModifierFlags::Control,
            58 | 61 => NSEventModifierFlags::Option,
            54 | 55 => NSEventModifierFlags::Command,
            57 => NSEventModifierFlags::CapsLock,
            _ => return false,
        };
        if !flags.contains(mask) || held.borrow().contains_key(&code) {
            GHOSTTY_KEY_ACTION_RELEASE
        } else {
            GHOSTTY_KEY_ACTION_PRESS
        }
    } else if up {
        GHOSTTY_KEY_ACTION_RELEASE
    } else if e.isARepeat() {
        GHOSTTY_KEY_ACTION_REPEAT
    } else {
        GHOSTTY_KEY_ACTION_PRESS
    };
    let released = if action == GHOSTTY_KEY_ACTION_RELEASE {
        match held.borrow_mut().remove(&code) {
            Some(value) => value,
            None => return false,
        }
    } else {
        None
    };
    let mut text = String::new();
    let mut unshifted = released;
    if !changed && !up {
        // Let dead keys and non-ASCII input sources enter AppKit's IME pipeline.
        let value = e
            .charactersByApplyingModifiers(
                flags & !(NSEventModifierFlags::Control | NSEventModifierFlags::Command),
            )
            .map(|s| s.to_string())
            .unwrap_or_default();
        let printable = |s: &str| {
            s.chars()
                .all(|c| !c.is_control() && !(0xf700..=0xf8ff).contains(&(c as u32)))
        };
        if printable(&value) {
            text = value;
        }
        if text.is_empty() && character_key(code) && !flags.contains(NSEventModifierFlags::Control)
        {
            return false;
        }
        unshifted = e
            .charactersByApplyingModifiers(NSEventModifierFlags::empty())
            .map(|s| s.to_string())
            .and_then(|s| s.chars().next())
            .filter(|c| !c.is_control());
    }
    let mut event = KeyEvent::press(key, &text);
    event.action = action;
    event.unshifted_codepoint = unshifted;
    event.modifiers = mods(flags);
    event.consumed_modifiers = if !text.is_empty() {
        event.modifiers & (GHOSTTY_MODS_SHIFT | GHOSTTY_MODS_ALT) as u16
    } else {
        0
    };
    session.key_input(&event);
    if action != GHOSTTY_KEY_ACTION_RELEASE {
        held.borrow_mut().insert(code, unshifted);
    }
    // Modifier changes must continue through AppKit, e.g. Caps Lock state/UI.
    !changed
}
fn mods(f: NSEventModifierFlags) -> u16 {
    [
        (NSEventModifierFlags::Shift, GHOSTTY_MODS_SHIFT),
        (NSEventModifierFlags::Control, GHOSTTY_MODS_CTRL),
        (NSEventModifierFlags::Option, GHOSTTY_MODS_ALT),
        (NSEventModifierFlags::Command, GHOSTTY_MODS_SUPER),
        (NSEventModifierFlags::CapsLock, GHOSTTY_MODS_CAPS_LOCK),
    ]
    .into_iter()
    .filter(|(mask, _)| f.contains(*mask))
    .fold(0, |m, (_, v)| m | v as u16)
}
fn reserved(code: u16, f: NSEventModifierFlags) -> bool {
    if code == 48 && f.contains(NSEventModifierFlags::Control) {
        return true;
    }
    if !f.contains(NSEventModifierFlags::Command) {
        return false;
    }
    // Workspace shortcuts plus native Copy, Paste and Select All retain priority.
    matches!(
        code,
        0 | 8 | 9 | 17 | 13 | 11 | 2 | 5 | 40 | 3 | 43 | 18..=23 | 25 | 26 | 28
    ) || (code == 36 && f.contains(NSEventModifierFlags::Shift))
        || (matches!(code, 123..=126) && f.contains(NSEventModifierFlags::Option))
}
fn defer_character_to_ime(code: u16, flags: NSEventModifierFlags) -> bool {
    if physical_keys() {
        return false;
    }
    character_key(code)
        && !ascii_input_source()
        && !flags.intersects(NSEventModifierFlags::Control | NSEventModifierFlags::Command)
}
fn character_key(code: u16) -> bool {
    matches!(code,0..=35|37..=47|50|65|67|69|75|78|81..=89|91|92|93|94|95)
}
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> *const std::ffi::c_void;
    fn TISGetInputSourceProperty(
        source: *const std::ffi::c_void,
        key: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    static kTISPropertyInputSourceIsASCIICapable: *const std::ffi::c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFBooleanGetValue(value: *const std::ffi::c_void) -> u8;
    fn CFRelease(value: *const std::ffi::c_void);
}
fn ascii_input_source() -> bool {
    // SAFETY: main thread only; copy is released, property is a borrowed CFBoolean.
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return true;
        }
        let value = TISGetInputSourceProperty(source, kTISPropertyInputSourceIsASCIICapable);
        let result = !value.is_null() && CFBooleanGetValue(value) != 0;
        CFRelease(source);
        result
    }
}
fn physical_key(code: u16) -> GhosttyKey {
    match code {
        0 => GHOSTTY_KEY_A,
        1 => GHOSTTY_KEY_S,
        2 => GHOSTTY_KEY_D,
        3 => GHOSTTY_KEY_F,
        4 => GHOSTTY_KEY_H,
        5 => GHOSTTY_KEY_G,
        6 => GHOSTTY_KEY_Z,
        7 => GHOSTTY_KEY_X,
        8 => GHOSTTY_KEY_C,
        9 => GHOSTTY_KEY_V,
        11 => GHOSTTY_KEY_B,
        12 => GHOSTTY_KEY_Q,
        13 => GHOSTTY_KEY_W,
        14 => GHOSTTY_KEY_E,
        15 => GHOSTTY_KEY_R,
        16 => GHOSTTY_KEY_Y,
        17 => GHOSTTY_KEY_T,
        18 => GHOSTTY_KEY_DIGIT_1,
        19 => GHOSTTY_KEY_DIGIT_2,
        20 => GHOSTTY_KEY_DIGIT_3,
        21 => GHOSTTY_KEY_DIGIT_4,
        22 => GHOSTTY_KEY_DIGIT_6,
        23 => GHOSTTY_KEY_DIGIT_5,
        24 => GHOSTTY_KEY_EQUAL,
        25 => GHOSTTY_KEY_DIGIT_9,
        26 => GHOSTTY_KEY_DIGIT_7,
        27 => GHOSTTY_KEY_MINUS,
        28 => GHOSTTY_KEY_DIGIT_8,
        29 => GHOSTTY_KEY_DIGIT_0,
        30 => GHOSTTY_KEY_BRACKET_RIGHT,
        31 => GHOSTTY_KEY_O,
        32 => GHOSTTY_KEY_U,
        33 => GHOSTTY_KEY_BRACKET_LEFT,
        34 => GHOSTTY_KEY_I,
        35 => GHOSTTY_KEY_P,
        36 => GHOSTTY_KEY_ENTER,
        37 => GHOSTTY_KEY_L,
        38 => GHOSTTY_KEY_J,
        39 => GHOSTTY_KEY_QUOTE,
        40 => GHOSTTY_KEY_K,
        41 => GHOSTTY_KEY_SEMICOLON,
        42 => GHOSTTY_KEY_BACKSLASH,
        43 => GHOSTTY_KEY_COMMA,
        44 => GHOSTTY_KEY_SLASH,
        45 => GHOSTTY_KEY_N,
        46 => GHOSTTY_KEY_M,
        47 => GHOSTTY_KEY_PERIOD,
        48 => GHOSTTY_KEY_TAB,
        49 => GHOSTTY_KEY_SPACE,
        50 => GHOSTTY_KEY_BACKQUOTE,
        51 => GHOSTTY_KEY_BACKSPACE,
        53 => GHOSTTY_KEY_ESCAPE,
        54 => GHOSTTY_KEY_META_RIGHT,
        55 => GHOSTTY_KEY_META_LEFT,
        56 => GHOSTTY_KEY_SHIFT_LEFT,
        57 => GHOSTTY_KEY_CAPS_LOCK,
        58 => GHOSTTY_KEY_ALT_LEFT,
        59 => GHOSTTY_KEY_CONTROL_LEFT,
        60 => GHOSTTY_KEY_SHIFT_RIGHT,
        61 => GHOSTTY_KEY_ALT_RIGHT,
        62 => GHOSTTY_KEY_CONTROL_RIGHT,
        64 => GHOSTTY_KEY_F17,
        65 => GHOSTTY_KEY_NUMPAD_DECIMAL,
        67 => GHOSTTY_KEY_NUMPAD_MULTIPLY,
        69 => GHOSTTY_KEY_NUMPAD_ADD,
        71 => GHOSTTY_KEY_NUMPAD_CLEAR,
        75 => GHOSTTY_KEY_NUMPAD_DIVIDE,
        76 => GHOSTTY_KEY_NUMPAD_ENTER,
        78 => GHOSTTY_KEY_NUMPAD_SUBTRACT,
        79 => GHOSTTY_KEY_F18,
        80 => GHOSTTY_KEY_F19,
        81 => GHOSTTY_KEY_NUMPAD_EQUAL,
        82 => GHOSTTY_KEY_NUMPAD_0,
        83 => GHOSTTY_KEY_NUMPAD_1,
        84 => GHOSTTY_KEY_NUMPAD_2,
        85 => GHOSTTY_KEY_NUMPAD_3,
        86 => GHOSTTY_KEY_NUMPAD_4,
        87 => GHOSTTY_KEY_NUMPAD_5,
        88 => GHOSTTY_KEY_NUMPAD_6,
        89 => GHOSTTY_KEY_NUMPAD_7,
        90 => GHOSTTY_KEY_F20,
        91 => GHOSTTY_KEY_NUMPAD_8,
        92 => GHOSTTY_KEY_NUMPAD_9,
        93 => GHOSTTY_KEY_INTL_YEN,
        94 => GHOSTTY_KEY_INTL_BACKSLASH,
        95 => GHOSTTY_KEY_NUMPAD_COMMA,
        96 => GHOSTTY_KEY_F5,
        97 => GHOSTTY_KEY_F6,
        98 => GHOSTTY_KEY_F7,
        99 => GHOSTTY_KEY_F3,
        100 => GHOSTTY_KEY_F8,
        101 => GHOSTTY_KEY_F9,
        103 => GHOSTTY_KEY_F11,
        105 => GHOSTTY_KEY_F13,
        106 => GHOSTTY_KEY_F16,
        107 => GHOSTTY_KEY_F14,
        109 => GHOSTTY_KEY_F10,
        111 => GHOSTTY_KEY_F12,
        113 => GHOSTTY_KEY_F15,
        114 => GHOSTTY_KEY_INSERT,
        115 => GHOSTTY_KEY_HOME,
        116 => GHOSTTY_KEY_PAGE_UP,
        117 => GHOSTTY_KEY_DELETE,
        118 => GHOSTTY_KEY_F4,
        119 => GHOSTTY_KEY_END,
        120 => GHOSTTY_KEY_F2,
        121 => GHOSTTY_KEY_PAGE_DOWN,
        122 => GHOSTTY_KEY_F1,
        123 => GHOSTTY_KEY_ARROW_LEFT,
        124 => GHOSTTY_KEY_ARROW_RIGHT,
        125 => GHOSTTY_KEY_ARROW_DOWN,
        126 => GHOSTTY_KEY_ARROW_UP,
        _ => GHOSTTY_KEY_UNIDENTIFIED,
    }
}
