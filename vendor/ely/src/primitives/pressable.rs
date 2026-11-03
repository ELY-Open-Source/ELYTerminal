use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, Task, Window, div,
};

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;
type Content = Box<dyn Fn(bool, &mut Window, &mut App) -> AnyElement>;

const LONG_PRESS: Duration = Duration::from_millis(500);

#[derive(Default)]
struct PressState {
    pressed: bool,
    long_fired: bool,
    _timer: Option<Task<()>>,
}

/// Pressed state and long press for any content.
#[derive(IntoElement)]
pub struct Pressable {
    id: ElementId,
    content: Content,
    on_press: Option<Handler>,
    on_long_press: Option<Handler>,
}

impl Pressable {
    /// `content` receives whether the pointer is down.
    pub fn new(
        id: impl Into<ElementId>,
        content: impl Fn(bool, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            content: Box::new(content),
            on_press: None,
            on_long_press: None,
        }
    }

    pub fn on_press(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_press = Some(Rc::new(handler));
        self
    }

    pub fn on_long_press(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_long_press = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Pressable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| PressState::default());
        let pressed = state.read(cx).pressed;
        let content = (self.content)(pressed, window, cx);
        let (down, up, out) = (state.clone(), state.clone(), state);
        let on_long_press = self.on_long_press;
        let on_press = self.on_press;

        div()
            .id(self.id)
            .child(content)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let timer = on_long_press.clone().map(|handler| {
                    let weak = down.downgrade();
                    window.spawn(cx, async move |cx| {
                        cx.background_executor().timer(LONG_PRESS).await;
                        let fired = cx.update(|window, cx| {
                            weak.update(cx, |press, cx| {
                                press.long_fired = press.pressed;
                                cx.notify();
                                press.long_fired
                            })
                            .map(|fire| fire.then(|| handler(window, cx)))
                        });
                        if let Err(error) = fired.and_then(|inner| inner) {
                            log::error!("pressable: long press lost its window: {error:#}");
                        }
                    })
                });
                down.update(cx, |press, cx| {
                    *press = PressState {
                        pressed: true,
                        long_fired: false,
                        _timer: timer,
                    };
                    cx.notify();
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                let released = up.update(cx, |press, cx| {
                    let short = press.pressed && !press.long_fired;
                    *press = PressState::default();
                    cx.notify();
                    short
                });
                if let Some(handler) = on_press.as_ref().filter(|_| released) {
                    handler(window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                out.update(cx, |press, cx| {
                    *press = PressState::default();
                    cx.notify();
                })
            })
    }
}
