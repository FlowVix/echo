use std::cell::RefCell;

use echo::tree;
use godot::prelude::*;

#[tree(Node2D())]
fn bar(v: Option<f32>) {
    STATE(lol = || vec![1, 2]);
    STATE(*ef = || 69);
    {
        lol.borrow_mut().push(69);
    }
}

fn main() {}
