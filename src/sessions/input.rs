use super::*;

impl Session {
    pub(crate) fn set_colors(&self, foreground: Option<&str>, background: Option<&str>) {
        fn rgb(value: &str) -> Option<Rgb> {
            let value = value.strip_prefix('#')?;
            if !value.is_ascii() || (value.len() != 6 && value.len() != 8) {
                return None;
            }
            Some(Rgb {
                r: u8::from_str_radix(&value[0..2], 16).ok()?,
                g: u8::from_str_radix(&value[2..4], 16).ok()?,
                b: u8::from_str_radix(&value[4..6], 16).ok()?,
            })
        }
        let (changed, events) = {
            let mut emulator = self.emulator.lock().unwrap();
            let changed = emulator.set_colors(foreground.and_then(rgb), background.and_then(rgb));
            (changed, emulator.backend_events())
        };
        self.handle_events(events);
        if changed {
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    pub(crate) fn input(&self, bytes: &[u8]) {
        if self.closed.load(Ordering::Acquire) || self.info.lock().unwrap().exit_code.is_some() {
            return;
        }
        if self.input.push(bytes).is_err() {
            let info = self.info();
            if let Some(warning) = info.input_warning {
                crate::notifications::record(
                    "app",
                    "warning",
                    Some(&info.id),
                    "Input not sent",
                    &warning,
                    &info.title,
                );
            }
            self.revision.fetch_add(1, Ordering::Release);
        }
    }

    pub(super) fn encoded_input(&self, encoded: Result<Vec<u8>, String>) {
        match encoded {
            Ok(bytes) if !bytes.is_empty() => self.input(&bytes),
            Ok(_) => {}
            Err(error) => self.fail(format!("Encoding terminal input: {error}")),
        }
    }
    /// Max lock wait and total encode+send time for input since the last call.
    /// Diagnostic only: the counters are reset by the display summary.
    pub(crate) fn input_probe(&self) -> (u64, u64, u64) {
        (
            self.input_count.swap(0, Ordering::Relaxed),
            self.input_wait_ns.swap(0, Ordering::Relaxed),
            self.input_total_ns.swap(0, Ordering::Relaxed),
        )
    }
    fn observe_input(&self, started: Instant, waited: Duration) {
        self.input_count.fetch_add(1, Ordering::Relaxed);
        self.input_wait_ns
            .fetch_max(waited.as_nanos() as u64, Ordering::Relaxed);
        self.input_total_ns
            .fetch_max(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
    pub(crate) fn text_input(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_text(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn key_input(&self, event: &termlevel_ghostty_vt::KeyEvent<'_>) {
        let started = Instant::now();
        if event.action != termlevel_ghostty_vt::raw::GHOSTTY_KEY_ACTION_RELEASE {
            self.input_epoch.fetch_add(1, Ordering::Release);
        }
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_key(event);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }

    /// Encodes mouse reports only when the terminal's current mode accepts
    /// them. The mode check and the encoding share one lock, so an event can
    /// never be classified under one mode and encoded under another. Every
    /// mouse entrance goes through this single ordered path.
    ///
    /// Returns `NotReporting` with the hold state observed under the same lock,
    /// which lets a history scroll avoid a second lock.
    pub(crate) fn mouse_reports(
        &self,
        shift: bool,
        events: &[(termlevel_ghostty_vt::MouseEvent, usize)],
        cw: u32,
        lh: u32,
    ) -> MouseReport {
        let started = Instant::now();
        let mut bytes = Vec::new();
        let waited;
        {
            let lock_at = Instant::now();
            let mut emulator = self.emulator.lock().unwrap();
            waited = lock_at.elapsed();
            if shift || !emulator.mouse_enabled() {
                return MouseReport::NotReporting {
                    held: emulator.render_held(),
                };
            }
            for (event, count) in events {
                for _ in 0..*count {
                    match emulator.encode_mouse(*event, cw, lh) {
                        Ok(part) => bytes.extend(part),
                        Err(error) => {
                            drop(emulator);
                            self.fail(format!("Encoding terminal input: {error}"));
                            return MouseReport::Encoded;
                        }
                    }
                }
            }
        }
        if !bytes.is_empty() {
            // All mouse entrances ask the display to follow before its next
            // published window. A rejected send stays visible; it is not a
            // delivery guarantee. A realistic event fits one message, so the
            // sequence is never split; only an absurd burst is chunked.
            self.input_epoch.fetch_add(1, Ordering::Release);
            if bytes.len() <= 1024 * 1024 {
                self.input(&bytes);
            } else {
                for chunk in bytes.chunks(1024 * 1024) {
                    self.input(chunk);
                }
            }
        }
        self.observe_input(started, waited);
        MouseReport::Encoded
    }
    pub(crate) fn paste(&self, text: &str) {
        let started = Instant::now();
        self.input_epoch.fetch_add(1, Ordering::Release);
        let lock_at = Instant::now();
        let encoded = self.emulator.lock().unwrap().encode_paste(text);
        let waited = lock_at.elapsed();
        self.encoded_input(encoded);
        self.observe_input(started, waited);
    }
    pub(crate) fn focus_input(&self, focused: bool) {
        let encoded = self.emulator.lock().unwrap().encode_focus(focused);
        self.encoded_input(encoded);
    }
}
