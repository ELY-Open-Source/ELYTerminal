use std::{future::Future, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Context, ElementId, IntoElement, ParentElement,
    Render, RenderOnce, Styled, Task, Window, div, prelude::*,
};

use super::ErrorBoundary;
use crate::{
    forms::Run,
    motion::{self, Spinner},
};

/// Shows a fallback while its content is on the way, then the content as it fades in, or an error view of the failure.
#[derive(IntoElement)]
pub struct Suspense {
    id: ElementId,
    content: Option<anyhow::Result<AnyElement>>,
    fallback: Option<AnyElement>,
    on_retry: Option<Run>,
}

impl Suspense {
    /// `None` while the content is on the way.
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        content: Option<anyhow::Result<E>>,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.map(|content| content.map(IntoElement::into_any_element)),
            fallback: None,
            on_retry: None,
        }
    }

    /// Shown while waiting instead of a centered spinner, such as a skeleton.
    pub fn fallback(mut self, fallback: impl IntoElement) -> Self {
        self.fallback = Some(fallback.into_any_element());
        self
    }

    pub fn on_retry(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Suspense {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        match self.content {
            None => self.fallback.unwrap_or_else(|| {
                div()
                    .flex()
                    .justify_center()
                    .py_8()
                    .child(Spinner::new((self.id.clone(), "waiting")))
                    .into_any_element()
            }),
            Some(Ok(content)) => div()
                .child(content)
                .with_animation(
                    (self.id, "ready"),
                    Animation::new(motion::duration(motion::BASE, cx))
                        .with_easing(motion::ease_out_cubic),
                    |ready, t| ready.opacity(t),
                )
                .into_any_element(),
            Some(Err(error)) => ErrorBoundary::new(self.id, Err::<AnyElement, _>(error))
                .when_some(self.on_retry, |boundary, run| {
                    boundary.on_retry(move |window, cx| run(window, cx))
                })
                .into_any_element(),
        }
    }
}

type Load<T> = Rc<dyn Fn() -> Task<anyhow::Result<T>>>;
type Show<T> = Rc<dyn Fn(&T, &mut Window, &mut App) -> AnyElement>;

/// Loads a value off the main thread and shows it through a `Suspense`: a spinner while it loads, then `show`'s view of it, or the error with Try again.
pub struct AsyncView<T> {
    id: ElementId,
    load: Load<T>,
    show: Show<T>,
    state: Option<anyhow::Result<T>>,
    _task: Task<()>,
}

impl<T: 'static> AsyncView<T> {
    /// `load` runs once now, and again on each Try again.
    pub fn new<F, E>(
        id: impl Into<ElementId>,
        load: impl Fn() -> F + 'static,
        show: impl Fn(&T, &mut Window, &mut App) -> E + 'static,
        cx: &mut Context<Self>,
    ) -> Self
    where
        F: Future<Output = anyhow::Result<T>> + Send + 'static,
        T: Send,
        E: IntoElement,
    {
        let load: Load<T> = {
            let executor = cx.background_executor().clone();
            Rc::new(move || executor.spawn(load()))
        };
        let task = Self::start(&load, cx);
        Self {
            id: id.into(),
            load,
            show: Rc::new(move |value, window, cx| show(value, window, cx).into_any_element()),
            state: None,
            _task: task,
        }
    }

    fn start(load: &Load<T>, cx: &mut Context<Self>) -> Task<()> {
        let running = load();
        cx.spawn(async move |view, cx| {
            let result = running.await;
            let landed = view.update(cx, |view, cx| {
                match &result {
                    Ok(_) => log::info!("async view {:?}: loaded", view.id),
                    Err(error) => log::error!("async view {:?}: {error:#}", view.id),
                }
                view.state = Some(result);
                cx.notify();
            });
            if landed.is_err() {
                log::debug!("async view: dropped before its value landed");
            }
        })
    }

    /// Loads anew, showing the fallback until the value lands.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        log::info!("async view {:?}: loading again", self.id);
        self.state = None;
        self._task = Self::start(&self.load, cx);
        cx.notify();
    }
}

impl<T: 'static> Render for AsyncView<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.state.as_ref().map(|state| match state {
            Ok(value) => Ok((self.show)(value, window, cx)),
            Err(error) => Err(anyhow::anyhow!("{error:#}")),
        });
        let view = cx.entity();
        Suspense::new(self.id.clone(), content)
            .on_retry(move |_, cx| view.update(cx, |view, cx| view.reload(cx)))
    }
}
