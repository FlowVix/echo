use std::cell::RefCell;

use echo::tree;
use godot::{
    classes::{Button, Control, PanelContainer},
    prelude::*,
};

macro_rules! __echo_deny {
    ($($t:tt)*) => {};
}
pub(crate) use __echo_deny;

#[tree(Node2D())]
fn bar() {
    Control..{
        PanelContainer..{
            foo()..{};
        };
        //
    };
}
#[tree(Control() in PanelContainer)]
fn foo() {
    {
        let n = __builder.node();
    }
}

fn main() {}
