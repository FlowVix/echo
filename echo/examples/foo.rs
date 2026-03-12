use echo::tree;
use godot::classes::{Button, Control, VBoxContainer};
use godot::prelude::*;

#[tree(Node(i32, f32))]
fn foo(v: f32) {
    BODY(69, 1.0);
}

#[tree(Node2D())]
fn bar(v: f32) {
    foo(v)..{
        let (mut v, ..) = ARGS;
        INIT(vov = v);
    };
}

fn main() {}
