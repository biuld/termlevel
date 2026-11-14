//! Debug-only native test driver. Exercises the actual AppKit field editor,
//! without adding composition injection to the production terminal API.
use nsh_core::prelude::*;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSTextField, NSTextInputClient, NSTextView, NSView,
};
use objc2_foundation::{NSNotFound, NSRange, NSString};

use std::cell::RefCell;
thread_local! {
    static VIEWS: RefCell<Vec<(String, ElementRef, ElementRef)>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn track(id: String, input: ElementRef, cursor: ElementRef) {
    VIEWS.with(|views| views.borrow_mut().push((id.clone(), input, cursor)));
    cleanup(move || VIEWS.with(|views| views.borrow_mut().retain(|(key, _, _)| key != &id)));
}

thread_local! {
    static PASTEBOARD:RefCell<Vec<objc2::rc::Retained<objc2_app_kit::NSPasteboardItem>>>=const{RefCell::new(Vec::new())};
}
#[derive(JsObject)]
pub struct InputGeometry {
    pub input_x: f64,
    pub input_y: f64,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

#[derive(JsObject)]
pub struct InputSnapshot {
    pub marked: bool,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn view_with_prefix(view: &NSView, prefix: &str) -> Option<Retained<NSView>> {
    if view
        .accessibilityIdentifier()
        .is_some_and(|id| id.to_string().starts_with(prefix))
    {
        return unsafe { Retained::retain(view as *const NSView as *mut NSView) };
    }
    let children = view.subviews();
    (0..children.count()).find_map(|i| view_with_prefix(&children.objectAtIndex(i), prefix))
}
fn text_editor(view: &NSView) -> Option<Retained<NSView>> {
    if view.downcast_ref::<NSTextField>().is_some() || view.downcast_ref::<NSTextView>().is_some() {
        return unsafe { Retained::retain(view as *const NSView as *mut NSView) };
    }
    let children = view.subviews();
    (0..children.count()).find_map(|i| text_editor(&children.objectAtIndex(i)))
}

#[module]
/// Debug-only AppKit composition driver for native acceptance tests.
pub struct NativeInputProbe;

#[module]
impl NativeInputProbe {
    /// Native host identity for local CPU sampling of terminal fixtures.
    pub fn process_id() -> u32 {
        std::process::id()
    }

    /// Deliver a pixel wheel event to the actual non-scrolling grid. NSH's
    /// test scroll primitive requires a ScrollView, which terminals do not use.
    pub async fn wheel(session_id: String, dy: i32) -> Result<(), String> {
        run_on_main_thread(move || {
            use objc2_app_kit::{NSEvent, NSScreen};
            use objc2_core_graphics::{CGEvent, CGEventField, CGScrollEventUnit};
            let mtm = MainThreadMarker::new().ok_or("not AppKit")?;
            let windows = NSApplication::sharedApplication(mtm).windows();
            let id = format!("terminal-grid-{session_id}");
            let grid = (0..windows.count())
                .find_map(|i| {
                    let root = windows.objectAtIndex(i).contentView()?;
                    view_with_prefix(&root, &id)
                })
                .ok_or("no terminal grid")?;
            let b = grid.bounds();
            let point = grid.convertPoint_toView(
                objc2_foundation::NSPoint::new(
                    b.origin.x + b.size.width / 2.0,
                    b.origin.y + b.size.height / 2.0,
                ),
                None,
            );
            let main_height = NSScreen::screens(mtm)
                .firstObject()
                .map_or(0.0, |s| s.frame().size.height);
            let event =
                CGEvent::new_scroll_wheel_event2(None, CGScrollEventUnit::Pixel, 1, -dy, 0, 0)
                    .ok_or("creating wheel event")?;
            CGEvent::set_location(
                Some(&event),
                objc2_foundation::NSPoint::new(point.x, main_height - point.y),
            );
            CGEvent::set_integer_value_field(
                Some(&event),
                CGEventField::ScrollWheelEventScrollPhase,
                2,
            );
            let event = NSEvent::eventWithCGEvent(&event).ok_or("creating NSEvent")?;
            grid.scrollWheel(&event);
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Number of actual drawRect calls in the app-owned terminal NSView.
    pub async fn surface_frames(session_id: String) -> Result<u32, String> {
        run_on_main_thread(move || {
            let mtm = MainThreadMarker::new().ok_or("not AppKit")?;
            let windows = NSApplication::sharedApplication(mtm).windows();
            let id = format!("terminal-grid-{session_id}");
            (0..windows.count())
                .find_map(|i| {
                    let root = windows.objectAtIndex(i).contentView()?;
                    let grid = view_with_prefix(&root, &id)?;
                    super::view::frames(&grid)
                })
                .ok_or_else(|| "no Termlevel native surface".to_owned())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Chooses the scripted NSH fallback versus actual AppKit hardware events.
    pub fn synthetic_keys(enabled: bool) {
        super::keyboard::set_synthetic_keys(enabled);
    }

    /// Encodes character keys through Ghostty while a non-ASCII input source is active.
    pub fn physical_keys(enabled: bool) {
        super::keyboard::set_physical_keys(enabled);
    }

    /// Reads the actual native pasteboard, preserving every item/type for restore.
    pub async fn pasteboard(action: String) -> Result<String, String> {
        run_on_main_thread(move || {
            use objc2::runtime::ProtocolObject;
            use objc2_app_kit::{
                NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting,
            };
            use objc2_foundation::NSArray;
            let pb = NSPasteboard::generalPasteboard();
            if action == "save" {
                let mut saved = Vec::new();
                if let Some(items) = pb.pasteboardItems() {
                    for i in 0..items.count() {
                        let item = items.objectAtIndex(i);
                        let copy = NSPasteboardItem::new();
                        let types = item.types();
                        for j in 0..types.count() {
                            let kind = types.objectAtIndex(j);
                            if let Some(data) = item.dataForType(&kind) {
                                copy.setData_forType(&data, &kind);
                            }
                        }
                        saved.push(copy);
                    }
                }
                PASTEBOARD.with(|s| *s.borrow_mut() = saved);
            } else if action == "restore" {
                PASTEBOARD.with(|s| {
                    let saved = s.borrow();
                    let objects: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = saved
                        .iter()
                        .map(|i| ProtocolObject::from_ref(&**i))
                        .collect();
                    pb.clearContents();
                    if !objects.is_empty() {
                        pb.writeObjects(&NSArray::from_slice(&objects));
                    }
                });
            }
            pb.stringForType(unsafe { NSPasteboardTypeString })
                .map(|s| s.to_string())
                .unwrap_or_default()
        })
        .await
        .map_err(|e| e.to_string())
    }
    /// Posts actual AppKit key down/repeat/up events through its local monitors.
    pub async fn key(
        action: String,
        code: u32,
        text: String,
        modifiers: u32,
    ) -> Result<(), String> {
        run_on_main_thread(move || {
            use objc2_app_kit::{NSEvent,NSEventType,NSEventModifierFlags};
            use objc2_foundation::NSPoint;
            let mtm=MainThreadMarker::new().ok_or("not AppKit")?;
            let app=NSApplication::sharedApplication(mtm);let windows=app.windows();
            let (window, editor)=(0..windows.count()).find_map(|i|{
                let window=windows.objectAtIndex(i);
                let root=window.contentView()?;
                let input=view_with_prefix(&root, "terminal-input-")?;
                let editor=text_editor(&input).unwrap_or(input);
                Some((window, editor))
            }).ok_or("no field editor")?;
            window.makeKeyAndOrderFront(None);
            if !window.makeFirstResponder(Some(&*editor)) {
                return Err("field editor rejected focus".to_owned());
            }
            let kind=if action=="release"{NSEventType::KeyUp}else{NSEventType::KeyDown};
            let string=NSString::from_str(&text);
            let event=NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(kind,NSPoint::new(0.0,0.0),NSEventModifierFlags::from_bits_retain(modifiers as usize),0.0,window.windowNumber(),None,&string,&string,action=="repeat",code as u16).ok_or("creating event")?;
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            // sendEvent runs local monitors before returning. postEvent drops
            // synthetic repeats and releases on this host.
            app.sendEvent(&event);
            Ok(())
        }).await.map_err(|e|e.to_string())?
    }

    /// Compares actual mounted input and output-cursor frames in root space.
    pub async fn geometry(session_id: String) -> Result<InputGeometry, String> {
        let (_, input, cursor) = VIEWS
            .with(|views| {
                views
                    .borrow()
                    .iter()
                    .find(|(id, _, _)| id == &session_id)
                    .cloned()
            })
            .ok_or("no registered view")?;
        let input = input.measure().await.ok_or("input not laid out")?;
        let cursor = cursor.measure().await.ok_or("cursor not laid out")?;
        Ok(InputGeometry {
            input_x: input.page.x.0 as f64,
            input_y: input.page.y.0 as f64,
            cursor_x: cursor.page.x.0 as f64,
            cursor_y: cursor.page.y.0 as f64,
        })
    }

    /// Reads the Rust model to distinguish marked text from PTY echo.
    pub fn output(session_id: String) -> Result<String, String> {
        use nsh_base::TerminalEmulator;
        let session = crate::sessions::manager()
            .lock()
            .unwrap()
            .get(&session_id)?;
        let emu = session.emulator.lock().unwrap();
        Ok((0..emu.line_count())
            .map(|i| emu.line(i).text())
            .collect::<Vec<_>>()
            .join("\n"))
    }
    /// Operates on the focused AppKit field editor. Native tests only.
    pub async fn edit(action: String, text: String) -> Result<InputSnapshot, String> {
        run_on_main_thread(move || {
            let mtm = MainThreadMarker::new().ok_or("not a native AppKit host")?;
            let app = NSApplication::sharedApplication(mtm);
            // Background development hosts intentionally have no key/main
            // window; locate their field editor without activating the app.
            let windows = app.windows();
            let responder = (0..windows.count())
                .find_map(|i| {
                    let window = windows.objectAtIndex(i);
                    window
                        .firstResponder()
                        .filter(|r| r.downcast_ref::<NSTextView>().is_some())
                })
                .ok_or("no focused AppKit field editor")?;
            let editor = responder.downcast_ref::<NSTextView>().unwrap();
            let no_range = NSRange::new(NSNotFound as usize, 0);
            let string = NSString::from_str(&text);
            // Both methods accept NSString and NSAttributedString. These are
            // the same NSTextInputClient entry points a system IME invokes.
            unsafe {
                match action.as_str() {
                    "mark" => editor.setMarkedText_selectedRange_replacementRange(
                        &string,
                        NSRange::new(text.encode_utf16().count(), 0),
                        no_range,
                    ),
                    "commit" => editor.insertText_replacementRange(&string, no_range),
                    "cancel" => {
                        editor.insertText_replacementRange(
                            &NSString::from_str(""),
                            editor.markedRange(),
                        );
                    }
                    "read" => {}
                    _ => return Err("unknown action".to_owned()),
                }
            }
            let rect = unsafe {
                editor.firstRectForCharacterRange_actualRange(
                    editor.selectedRange(),
                    std::ptr::null_mut(),
                )
            };
            Ok(InputSnapshot {
                marked: editor.hasMarkedText(),
                text: editor.string().to_string(),
                x: rect.origin.x,
                y: rect.origin.y,
                width: rect.size.width,
                height: rect.size.height,
            })
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
