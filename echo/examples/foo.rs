use std::cell::RefCell;

use echo::tree;
use godot::{classes::Button, prelude::*};

macro_rules! __echo_deny {
    ($($t:tt)*) => {};
}

#[tree(Node2D())]
fn bar() -> f32 {
    STATE(lol = || vec![1, 2]);
    STATE(*ef = || 69);
    {
        lol.borrow_mut().push(69);
    }
    // Button..{
    //     //
    // };
}

fn main() {}
