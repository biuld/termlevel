//! Platform-neutral latest-scene channel and native component binding.
//! Slots own immutable scenes only, never a session or native UI object.
use super::paint::Scene;
use nsh_core::{JsonValue, prelude::*};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

pub(super) const COMPONENT_NAME: &str = "termlevel.terminal-surface";
type Slots = HashMap<u64, Weak<Slot>>;
fn slots() -> &'static Mutex<Slots> {
    static SLOTS: OnceLock<Mutex<Slots>> = OnceLock::new();
    SLOTS.get_or_init(Mutex::default)
}
pub(super) struct Slot {
    id: u64,
    scene: Mutex<Option<Arc<Scene>>>,
    revision: AtomicU64,
}
impl Drop for Slot {
    fn drop(&mut self) {
        slots().lock().unwrap().remove(&self.id);
    }
}
#[derive(Clone)]
pub(super) struct Publisher(Arc<Slot>);
impl Publisher {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let slot = Arc::new(Slot {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            scene: Mutex::new(None),
            revision: AtomicU64::new(0),
        });
        slots()
            .lock()
            .unwrap()
            .insert(slot.id, Arc::downgrade(&slot));
        Self(slot)
    }
    pub fn publish(&self, scene: Arc<Scene>) -> JsonValue {
        *self.0.scene.lock().unwrap() = Some(scene);
        let rev = self.0.revision.fetch_add(1, Ordering::Relaxed) + 1;
        JsonValue::Array(vec![self.0.id.into(), rev.into()])
    }
}

impl Slot {
    pub(super) fn latest(&self) -> Option<Arc<Scene>> {
        self.scene.lock().unwrap().clone()
    }
}

pub(super) fn resolve(value: Option<&JsonValue>) -> Option<Arc<Slot>> {
    let id = value.and_then(|v| v.get(0)).and_then(JsonValue::as_u64)?;
    slots().lock().unwrap().get(&id).and_then(Weak::upgrade)
}

/// Bind owned Rust scenes to a platform's registered native component.
/// Only the slot/revision token crosses the NSH property boundary.
pub(super) fn element(
    scene: impl Fn() -> Scene + 'static,
    label: impl Fn() -> SharedString + 'static,
) -> impl IntoElement {
    let publisher = Publisher::new();
    nsh_core::element::native(COMPONENT_NAME)
        .prop("frame", move || publisher.publish(Arc::new(scene())))
        .absolute()
        .inset_0()
        .role(AccessibilityRole::Text)
        .label(label)
        .pointer_events(PointerEvents::None)
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session_view::{paint::PaintConfig, window::Window},
        terminal::GhosttyEmulator,
    };
    use nsh_base::{TerminalEmulator, TerminalSize};
    fn config() -> PaintConfig {
        PaintConfig {
            cw: 10.0,
            lh: 20.0,
            family: "Menlo".into(),
            size: 13.0,
            foreground: nsh_core::color::rgb(0xffffff).into(),
            background: nsh_core::color::rgb(0).into(),
            selection: nsh_core::color::rgb(0xffff00).into(),
            search: nsh_core::color::rgb(0xff00ff).into(),
        }
    }
    fn scene() -> Scene {
        let mut emulator = GhosttyEmulator::new(TerminalSize::new(4, 1), 10);
        emulator.feed(b"\x1b[48;2;255;0;0m  A \x1b[0m");
        let snap = emulator
            .begin_window(&mut Window::default(), false, None)
            .unwrap()
            .finish();
        Scene {
            snap,
            config: config(),
            hits: Vec::new(),
            current: 0,
        }
    }
    #[test]
    fn publisher_is_latest_frame_only_and_lifetime_does_not_own_a_session() {
        let publisher = Publisher::new();
        let id = publisher.0.id;
        let first = publisher.publish(Arc::new(scene()));
        let old = publisher
            .0
            .scene
            .lock()
            .unwrap()
            .as_ref()
            .map(Arc::downgrade)
            .unwrap();
        let next = publisher.publish(Arc::new(scene()));
        assert_eq!(first[0], next[0]);
        assert_ne!(first[1], next[1]);
        assert!(old.upgrade().is_none(), "pending frames must coalesce");
        let receiver = resolve(Some(&next)).unwrap();
        assert!(Arc::ptr_eq(
            &receiver.latest().unwrap(),
            publisher.0.scene.lock().unwrap().as_ref().unwrap(),
        ));
        drop(publisher);
        assert!(
            resolve(Some(&next)).is_some(),
            "native host retains the slot"
        );
        drop(receiver);
        assert!(!slots().lock().unwrap().contains_key(&id));
        assert!(
            resolve(Some(&next)).is_none(),
            "expired tokens cannot rebind"
        );
        assert!(resolve(None).is_none());
    }

    #[test]
    fn semantic_adapter_records_no_drawing_operations() {
        let scene = scene();
        let drawing = crate::session_view::paint::semantics(&scene.snap, 10.0, 20.0).finish();
        assert!(drawing.ops.is_empty());
        assert!(!drawing.nodes.is_empty());
    }
}
