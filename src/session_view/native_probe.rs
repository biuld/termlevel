//! Debug-only native test driver. Exercises the actual AppKit field editor,
//! without adding composition injection to the production terminal API.
use nsh_core::prelude::*;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSTextInputClient, NSTextView};
use objc2_foundation::{NSNotFound, NSRange, NSString};

use std::cell::RefCell;
thread_local! {
    static VIEWS: RefCell<Vec<(String, ElementRef, ElementRef)>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn track(id: String, input: ElementRef, cursor: ElementRef) {
    VIEWS.with(|views| views.borrow_mut().push((id.clone(), input, cursor)));
    cleanup(move || VIEWS.with(|views| views.borrow_mut().retain(|(key, _, _)| key != &id)));
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

#[module]
/// Debug-only AppKit composition driver for native acceptance tests.
pub struct NativeInputProbe;

#[module]
impl NativeInputProbe {
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
