use echo::tree;
use godot::classes::{Button, Control, Label, VBoxContainer};
use godot::prelude::*;

#[tree(Node(i32, f32))]
fn foo(v: f32) {
    BODY(69, 1.0);
}

#[tree(Node2D())]
fn bar(v: Option<f32>) {
    match v {
        Some(v) => {
            Label..{
                INIT(text = v);
            };
        }
        None => {}
    }
}

fn main() {}
