//! Synchronous clipboard effects. Hosts provide a thread-safe, bounded adapter;
//! the C request and its reply never escape the callback lifetime.
use super::{Error, Result, Terminal, check, sys};
use std::{ffi::c_void, sync::Arc};

/// Synchronous host adapter. Implementations must be bounded and must not
/// reenter the terminal or wait for a thread that may be locking it.
pub trait Clipboard: Send + Sync {
    fn read(&self) -> Result<String>;
    fn write(&self, text: &str) -> Result<()>;
}

impl Terminal {
    pub fn set_clipboard(&mut self, clipboard: Arc<dyn Clipboard>) -> Result<()> {
        self.events.get_mut().clipboard = Some(clipboard);
        unsafe {
            // SAFETY: exact callback function signatures; userdata lives with self.
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_CLIPBOARD_WRITE,
                on_write as *const () as *const c_void,
            ))?;
            check(sys::ghostty_terminal_set(
                self.handle.0,
                sys::GHOSTTY_TERMINAL_OPT_CLIPBOARD_READ,
                on_read as *const () as *const c_void,
            ))
        }
    }
}
unsafe fn bytes<'a>(s: sys::GhosttyString) -> &'a [u8] {
    if s.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(s.ptr, s.len) }
    }
}
unsafe extern "C" fn on_write(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    request: *const sys::GhosttyClipboardWrite,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        let request = &*request;
        let Some(reply) = request.reply else { return };
        let mut result = sys::GHOSTTY_CLIPBOARD_WRITE_RESULT_DENIED;
        if request.location == sys::GHOSTTY_CLIPBOARD_LOCATION_STANDARD
            && let Some(host) = (&*userdata.cast::<super::Events>()).clipboard.clone()
        {
            let contents = if request.contents_len == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(request.contents, request.contents_len)
            };
            let text = contents.iter().find(|item| {
                matches!(
                    bytes(item.mime),
                    b"text/plain" | b"text/plain;charset=utf-8" | b"text/plain;charset=UTF-8"
                )
            });
            let written = if contents.is_empty() {
                host.write("")
            } else if let Some(text) = text {
                if text.data.len > 1024 * 1024 {
                    Err(Error(sys::GHOSTTY_LIMIT_EXCEEDED))
                } else {
                    std::str::from_utf8(bytes(text.data))
                        .map_err(|_| Error(sys::GHOSTTY_INVALID_VALUE))
                        .and_then(|text| host.write(text))
                }
            } else {
                Err(Error(sys::GHOSTTY_UNSUPPORTED))
            };
            result = if written.is_ok() {
                sys::GHOSTTY_CLIPBOARD_WRITE_RESULT_SUCCESS
            } else {
                sys::GHOSTTY_CLIPBOARD_WRITE_RESULT_DENIED
            };
        }
        let answer = sys::GhosttyClipboardWriteReply {
            size: size_of::<sys::GhosttyClipboardWriteReply>(),
            result,
            remember: false,
        };
        reply(request, &answer);
    }));
}
unsafe extern "C" fn on_read(
    _: sys::GhosttyTerminal,
    userdata: *mut c_void,
    request: *const sys::GhosttyClipboardRead,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        let request = &*request;
        let Some(reply) = request.reply else { return };
        let host = (&*userdata.cast::<super::Events>()).clipboard.clone();
        let mimes = if request.mimes_len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(request.mimes, request.mimes_len)
        };
        let supported = request.location == sys::GHOSTTY_CLIPBOARD_LOCATION_STANDARD
            && (mimes.is_empty()
                || mimes.iter().any(|m| {
                    matches!(
                        bytes(*m),
                        b"text/plain" | b"text/plain;charset=utf-8" | b"text/plain;charset=UTF-8"
                    )
                }));
        let text = if supported {
            host.ok_or(Error(sys::GHOSTTY_REJECTED))
                .and_then(|host| host.read())
        } else {
            Err(Error(sys::GHOSTTY_UNSUPPORTED))
        };
        let (result, text) = match text {
            Ok(text) if text.len() <= 1024 * 1024 => {
                (sys::GHOSTTY_CLIPBOARD_READ_RESULT_SUCCESS, text)
            }
            _ => (sys::GHOSTTY_CLIPBOARD_READ_RESULT_DENIED, String::new()),
        };
        let mime = sys::GhosttyString {
            ptr: b"text/plain".as_ptr(),
            len: 10,
        };
        let contents = sys::GhosttyClipboardContent {
            mime,
            data: sys::GhosttyString {
                ptr: text.as_ptr(),
                len: text.len(),
            },
        };
        let answer = sys::GhosttyClipboardReadReply {
            size: size_of::<sys::GhosttyClipboardReadReply>(),
            result,
            contents: &contents,
            contents_len: usize::from(
                result == sys::GHOSTTY_CLIPBOARD_READ_RESULT_SUCCESS && !request.list,
            ),
            available: &mime,
            available_len: usize::from(result == sys::GHOSTTY_CLIPBOARD_READ_RESULT_SUCCESS),
            remember: false,
        };
        reply(request, &answer);
    }));
}
