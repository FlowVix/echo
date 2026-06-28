use std::{any::Any, cell::RefCell, rc::Rc};

use ahash::{AHashMap, AHashSet, HashMapExt, HashSetExt};
use godot::{
    classes::Node,
    obj::{Gd, Inherits},
    prelude::*,
};
use nohash_hasher::{IntMap, IntSet};
use smallvec::{SmallVec, smallvec};

use crate::builder::Builder;

pub struct MapItem {
    pub(crate) node: Gd<Node>,
    pub(crate) signals: SmallVec<[(&'static str, Callable); 4]>,
}

pub struct MemoItem {
    pub(crate) value: Box<dyn Any>,
    pub(crate) retain_ids: IntSet<u64>,
    pub(crate) retain_signals: AHashSet<(u64, &'static str)>,
    pub(crate) next_idx: i32,
}

pub struct Context {
    pub(crate) root: Gd<Node>,
    pub(crate) used_ids: IntSet<u64>,
    pub(crate) used_signals: AHashSet<(u64, &'static str)>,
    pub(crate) map: IntMap<u64, MapItem>,
    pub(crate) state_map: IntMap<u64, Box<dyn Any>>,
    pub(crate) memo_map: IntMap<u64, MemoItem>,
    pub(crate) signal_runs: AHashMap<(Gd<Node>, &'static str), SmallVec<[Variant; 4]>>,
    pub(crate) force_memo: bool,
}

pub struct App<R: Inherits<Node>, S> {
    pub(crate) root: Gd<R>,
    pub(crate) func: Box<dyn FnMut(Builder<R>, &mut S) -> Builder<R> + 'static>,
    pub(crate) ctx: Rc<RefCell<Context>>,
    pub(crate) ran: bool,
}

impl<R: Inherits<Node>, S> App<R, S> {
    pub fn new<F: FnMut(Builder<R>, &mut S) -> Builder<R> + 'static>(
        mut root: Gd<R>,
        func: F,
    ) -> Self {
        root.upcast_mut()
            .add_user_signal_ex("__echo_rerun")
            .arguments(
                &varray![&vdict! { "name" => "force_memo", "type" => VariantType::BOOL }]
                    .upcast_any_array(),
            )
            .done();
        Self {
            ctx: Rc::new(RefCell::new(Context {
                used_ids: IntSet::new(),
                map: IntMap::new(),
                state_map: IntMap::new(),
                memo_map: IntMap::new(),
                signal_runs: AHashMap::new(),
                root: root.clone().upcast(),
                used_signals: AHashSet::new(),
                force_memo: false,
            })),
            root,
            func: Box::new(func),
            ran: false,
        }
    }
    pub fn run(&mut self, state: &mut S, force_memo: bool) {
        {
            let ctx = &mut *self.ctx.borrow_mut();
            ctx.used_ids.clear();
            ctx.used_signals.clear();
            ctx.force_memo = force_memo;
        }
        let path = smallvec![];
        let cached_total_id = ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&path);

        (self.func)(
            Builder {
                node: self.root.clone(),
                next_idx: 0,
                ctx: self.ctx.clone(),
                new: !self.ran,
                next_push: 0,
                path,
                cached_total_id,
                memo_stack: vec![],
            },
            state,
        );
        self.ran = true;
        let ctx = &mut *self.ctx.borrow_mut();
        ctx.map.retain(|k, item| {
            if ctx.used_ids.contains(k) {
                for (s, c) in &item.signals {
                    if !ctx.used_signals.contains(&(*k, *s)) {
                        item.node.disconnect(*s, c);
                    }
                }

                true
            } else {
                item.node.queue_free();
                false
            }
        });
        ctx.state_map.retain(|k, _| ctx.used_ids.contains(k));
        ctx.memo_map.retain(|k, _| ctx.used_ids.contains(k));
        ctx.signal_runs.clear();
    }
}
