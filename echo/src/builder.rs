use std::{
    any::Any,
    cell::RefCell,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    panic::Location,
    rc::Rc,
};

use godot::{
    classes::{Button, Control, Label, Node, VBoxContainer, control::SizeFlags},
    global::godot_warn,
    meta::AsArg,
    obj::{Gd, Inherits, NewAlloc, WithSignals},
    prelude::*,
};
use smallbox::SmallBox;
use smallvec::{SmallVec, smallvec};

use crate::app::{Context, MapItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathElem {
    Inc(u64),
    Hash(u64),
}

pub struct Builder<P: Inherits<Node>> {
    pub(crate) node: Gd<P>,
    pub(crate) next_idx: i32,
    pub(crate) next_push: u64,
    pub(crate) path: SmallVec<[PathElem; 8]>,
    pub(crate) cached_total_id: u64,
    pub(crate) new: bool,
    pub(crate) ctx: Rc<RefCell<Context>>,
}

impl<P: Inherits<Node>> Builder<P> {
    // pub fn with_node(mut self, cb: impl FnOnce(&mut Gd<P>)) -> Self {
    //     cb(&mut self.node);
    //     self
    // }
    #[inline]
    #[doc(hidden)]
    pub fn __child<C: Inherits<Node> + NewAlloc>(
        mut self,
        cb: impl FnOnce(Builder<C>) -> Builder<C>,
    ) -> Builder<P> {
        let mut path = self.path;
        path.push(PathElem::Inc(self.next_push));
        self.next_push += 1;

        let mut ctx_b = self.ctx.borrow_mut();

        let cached_total_id = ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&path);
        ctx_b.used_ids.insert(cached_total_id);

        let child_b = match ctx_b.map.get(&cached_total_id) {
            Some(existing) => {
                let existing = existing.node.clone().cast::<C>();
                self.node.upcast_mut().move_child(&existing, self.next_idx);
                self.next_idx = existing.upcast_ref().get_index() + 1;
                Builder {
                    node: existing,
                    next_idx: 0,
                    next_push: 0,
                    path,
                    cached_total_id,
                    new: false,
                    ctx: self.ctx.clone(),
                }
            }
            None => {
                let new = C::new_alloc();
                self.node.upcast_mut().add_child(&new);
                self.node.upcast_mut().move_child(&new, self.next_idx);
                self.next_idx = new.upcast_ref().get_index() + 1;
                ctx_b.map.insert(
                    cached_total_id,
                    MapItem {
                        node: new.clone().upcast(),
                        signals: smallvec![],
                    },
                );
                Builder {
                    node: new,
                    next_idx: 0,
                    next_push: 0,
                    path,
                    cached_total_id,
                    new: true,
                    ctx: self.ctx.clone(),
                }
            }
        };
        drop(ctx_b);

        let mut child_b = cb(child_b);
        child_b.path.pop();

        self.path = child_b.path;
        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __under_explicit(mut self, id: impl Hash, cb: impl FnOnce(Self) -> Self) -> Self {
        let mut path = self.path;
        path.push(PathElem::Hash(
            ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&id),
        ));

        let cached_total_id = ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&path);

        let mut inner_b = cb(Builder {
            node: self.node,
            next_idx: self.next_idx,
            next_push: 0,
            path,
            cached_total_id,
            new: self.new,
            ctx: self.ctx,
        });
        inner_b.path.pop();

        self.path = inner_b.path;
        self.node = inner_b.node;
        self.ctx = inner_b.ctx;
        self.next_idx = inner_b.next_idx;
        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_prop<T: ToGodot>(mut self, prop: &str, value: T) -> Self {
        self.node.upcast_mut().set(prop, &value.to_variant());
        self
    }
    #[inline]
    #[doc(hidden)]
    #[track_caller]
    pub fn __signal(mut self, s: &'static str, cb: impl FnOnce(&[Variant])) -> Self {
        let mut ctx = self.ctx.borrow_mut();
        ctx.used_signals.insert((self.cached_total_id, s));

        if !ctx.map[&self.cached_total_id]
            .signals
            .iter()
            .any(|v| v.0 == s)
        {
            let ctx_clone = self.ctx.clone();
            let node_clone = self.node.clone().upcast();

            let callable = Callable::from_fn(
                format!("Signal callable at `{}`", Location::caller()),
                move |args| {
                    {
                        let ctx = &mut *ctx_clone.borrow_mut();
                        ctx.signal_runs.insert(
                            (node_clone.clone(), s),
                            args.iter().map(|v| v.to_variant()).collect(),
                        );
                        ctx.root.clone()
                    }
                    .emit_signal("__echo_rerun", &[]);
                },
            );
            self.node.upcast_mut().connect(s, &callable);

            ctx.map
                .get_mut(&self.cached_total_id)
                .unwrap()
                .signals
                .push((s, callable));
        }
        if let Some(args) = ctx
            .signal_runs
            .get(&(self.node.clone().upcast(), s))
            .cloned()
        {
            drop(ctx);
            cb(&args);
            self
        } else {
            drop(ctx);
            self
        }
    }
    #[inline]
    pub fn cast<T: Inherits<Node> + Inherits<P>>(self) -> Builder<T>
// where
    //     P: Inherits<T>,
    //     T: Inherits<Node>,
    {
        Builder {
            node: self.node.cast(),
            next_idx: self.next_idx,
            next_push: self.next_push,
            path: self.path,
            cached_total_id: self.cached_total_id,
            new: self.new,
            ctx: self.ctx,
        }
    }
    #[inline]
    pub fn upcast<T: Inherits<Node>>(self) -> Builder<T>
    where
        P: Inherits<T>,
    {
        Builder {
            node: self.node.upcast(),
            next_idx: self.next_idx,
            next_push: self.next_push,
            path: self.path,
            cached_total_id: self.cached_total_id,
            new: self.new,
            ctx: self.ctx,
        }
    }
    #[inline]
    pub fn node(&self) -> Gd<P> {
        self.node.clone()
    }
    #[inline]
    pub fn init(&self) -> bool {
        self.new
    }
}

impl<P: Inherits<Control> + Inherits<Node>> Builder<P> {
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_color_override(mut self, prop: &str, value: Color) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_color_override(prop, value);

        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_constant_override(mut self, prop: &str, value: i32) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_constant_override(prop, value);

        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_font_override(
        mut self,
        prop: &str,
        value: impl AsArg<Gd<godot::classes::Font>>,
    ) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_font_override(prop, value);

        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_font_size_override(mut self, prop: &str, value: i32) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_font_size_override(prop, value);

        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_icon_override(
        mut self,
        prop: &str,
        value: impl AsArg<Gd<godot::classes::Texture2D>>,
    ) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_icon_override(prop, value);

        self
    }
    #[inline]
    #[doc(hidden)]
    pub fn __set_theme_stylebox_override(
        mut self,
        prop: &str,
        value: impl AsArg<Gd<godot::classes::StyleBox>>,
    ) -> Self {
        self.node
            .upcast_mut::<Control>()
            .add_theme_stylebox_override(prop, value);

        self
    }
}
