//! AppKit native component host. Does not own PTYs or terminal state.
use super::renderer::Renderer;
use crate::session_view::presentation::{self, COMPONENT_NAME, Slot};
use nsh_core::JsonValue;
use nsh_platform_macos::{Components, NativeComponent};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSGraphicsContext, NSView};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(super) fn register(components: &Components) {
    components.register(
        COMPONENT_NAME,
        Rc::new(|_| {
            let mtm = MainThreadMarker::new().expect("native components mount on main thread");
            let view = Surface::alloc(mtm).set_ivars(Ivars::default());
            // SAFETY: initialize the allocated NSView subclass once.
            let view: Retained<Surface> =
                unsafe { msg_send![super(view), initWithFrame: NSRect::ZERO] };
            view.setWantsLayer(true);
            Box::new(Component(view))
        }),
    );
}
struct Component(Retained<Surface>);
impl NativeComponent for Component {
    fn view(&self) -> Retained<NSView> {
        self.0.clone().into_super()
    }
    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        if name != "frame" {
            return;
        }
        let slot = presentation::resolve(value);
        *self.0.ivars().slot.borrow_mut() = slot;
        self.0.setNeedsDisplay(true);
    }
}
#[derive(Default)]
struct Ivars {
    frames: Cell<u32>,
    slot: RefCell<Option<Arc<Slot>>>,
    renderer: RefCell<Renderer>,
}
define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "TermlevelTerminalSurface"]
    #[ivars = Ivars]
    struct Surface;
    unsafe impl NSObjectProtocol for Surface {}
    impl Surface {
        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> bool { true }
        // The NSH grid parent owns all pointer handling. This drawing view must
        // not become a second mouse responder or steal the field editor.
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> { None }
        #[unsafe(method(drawRect:))]
        fn draw(&self, _rect: NSRect) {
            let scene = self.ivars().slot.borrow().as_ref()
                .and_then(|s| s.latest());
            let (Some(scene), Some(context)) = (scene, NSGraphicsContext::currentContext()) else { return; };
            self.ivars().renderer.borrow_mut().draw(&context.CGContext(), &scene);
            self.ivars().frames.set(self.ivars().frames.get().saturating_add(1));
        }
    }
);

#[cfg(debug_assertions)]
pub(super) fn frames(view: &NSView) -> Option<u32> {
    if let Some(surface) = view.downcast_ref::<Surface>() {
        return Some(surface.ivars().frames.get());
    }
    let children = view.subviews();
    (0..children.count()).find_map(|i| frames(&children.objectAtIndex(i)))
}
