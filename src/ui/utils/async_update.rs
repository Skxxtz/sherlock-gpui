use std::{rc::Rc, sync::Arc};

use gpui::{App, AppContext, AsyncApp, Entity, Task, WeakEntity};

use crate::launcher::LauncherConfig;

pub trait Fetchable: Sized + Send + 'static {
    type Error: Send;
    async fn fetch(
        launcher: &Arc<LauncherConfig>,
        old: Option<Rc<Self>>,
    ) -> Result<Option<Rc<Self>>, Self::Error>;
}

pub trait AsyncUpdate {
    fn update_async(&self, launcher: Arc<LauncherConfig>, cx: &mut impl AppContext);
}

#[derive(Clone)]
pub struct AsyncUpdateEntity<T: Fetchable> {
    entity: Entity<AsyncUpdateEntityInner<T>>,
}

impl<T: Fetchable> AsyncUpdateEntity<T> {
    pub fn new(cx: &mut impl AppContext) -> Self {
        Self {
            entity: cx.new(|_| AsyncUpdateEntityInner::default()),
        }
    }

    pub fn read<'a>(&self, cx: &'a App) -> &'a Result<Option<Rc<T>>, T::Error> {
        &self.entity.read(cx).data
    }

    pub fn read_with<R, C: AppContext>(
        &self,
        cx: &C,
        f: impl FnOnce(&Result<Option<Rc<T>>, T::Error>, &App) -> R,
    ) -> R {
        self.entity.read_with(cx, |this, cx| f(&this.data, cx))
    }

    pub fn is_valid(&self, cx: &impl AppContext) -> bool {
        self.entity
            .read_with(cx, |this, _| this.data.as_ref().is_ok_and(|i| i.is_some()))
    }
}

struct AsyncUpdateEntityInner<T: Fetchable> {
    task: Option<Task<()>>,
    data: Result<Option<Rc<T>>, T::Error>,
}

impl<T: Fetchable> Default for AsyncUpdateEntityInner<T> {
    fn default() -> Self {
        Self {
            task: None,
            data: Ok(None),
        }
    }
}

impl<T: Fetchable> AsyncUpdate for AsyncUpdateEntity<T> {
    fn update_async(&self, launcher: Arc<LauncherConfig>, cx: &mut impl AppContext) {
        self.entity.update(cx, |this, cx| {
            // reset task
            this.task = None;
            this.task = Some(cx.spawn(
                |weak_self: WeakEntity<AsyncUpdateEntityInner<T>>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    async move {
                        let old = weak_self.upgrade().and_then(|this| {
                            this.read_with(&cx, |data, _| {
                                data.data.as_ref().ok().and_then(|i| i.clone())
                            })
                        });
                        let res = T::fetch(&launcher, old).await;
                        let _ = weak_self.update(&mut cx, |this, cx| {
                            this.data = res;
                            cx.notify();
                        });
                    }
                },
            ));
        });
    }
}
