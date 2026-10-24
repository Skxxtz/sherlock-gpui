use std::time::Duration;

use gpui::{App, AppContext, AsyncApp, Context, Entity, Task, WeakEntity};

#[derive(Clone)]
pub struct TimeoutCaller<T: 'static> {
    inner: Entity<TimeoutInner<T>>,
    repeat: bool,
}

pub struct TimeoutInner<T> {
    task: Option<Task<()>>,
    data: T,
}

#[allow(dead_code)]
impl<T: 'static> TimeoutCaller<T> {
    pub fn new(data: T, cx: &mut impl AppContext) -> Self {
        Self {
            inner: cx.new(|_| TimeoutInner { task: None, data }),
            repeat: false,
        }
    }

    pub fn start<F, R>(&self, duration: Duration, cx: &mut App, f: F)
    where
        F: Fn(&mut T, &mut Context<TimeoutInner<T>>) -> R + 'static,
    {
        // debounce if timer already exists
        if self.inner.read(cx).task.is_some() || duration.is_zero() {
            return;
        }

        let repeat = self.repeat;
        self.inner.update(cx, |this, cx| {
            this.task = Some(cx.spawn(
                move |weak_self: WeakEntity<TimeoutInner<T>>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        loop {
                            cx.background_executor().timer(duration).await;
                            let ok = weak_self
                                .update(&mut cx, |this, cx| {
                                    f(&mut this.data, cx);
                                    cx.notify();
                                })
                                .is_ok();

                            if !ok || !repeat {
                                break;
                            }
                        }

                        let _ = weak_self.update(&mut cx, |this, _| {
                            this.task = None;
                        });
                    }
                },
            ));
        });
    }

    /// Removes a running timer if one exists
    pub fn stop(&self, cx: &mut App) {
        self.inner.update(cx, |this, _| {
            this.task = None;
        })
    }

    /// Checks if a timer is currently running
    pub fn is_running(&self, cx: &App) -> bool {
        self.inner.read(cx).task.is_some()
    }

    /// Sets the repeat flag if the task should repeat
    pub fn repeat(&mut self) {
        self.repeat = true;
    }

    pub fn read<'a>(&self, cx: &'a App) -> &'a T {
        &self.inner.read(cx).data
    }
}

pub trait Timeout: 'static {
    fn update_task(&self) -> &Option<Task<()>>;
    fn update_task_mut(&mut self) -> &mut Option<Task<()>>;

    fn start_timer<F, R>(&mut self, duration: Duration, cx: &mut Context<Self>, f: F)
    where
        Self: Sized,
        F: FnOnce(&mut Self, &mut Context<Self>) -> R + 'static,
    {
        if self.update_task().is_some() {
            return;
        }
        *self.update_task_mut() = Some(cx.spawn(
            move |weak_self: gpui::WeakEntity<Self>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    cx.background_executor().timer(duration).await;
                    let _ = weak_self.update(&mut cx, |this, cx| {
                        f(this, cx);
                        cx.notify();
                        *this.update_task_mut() = None;
                    });
                }
            },
        ));
    }
}
