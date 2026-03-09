#![deny(unused_must_use, unnameable_types)]
#![allow(clippy::too_many_arguments, clippy::single_match)]

mod app;
mod builder;

pub use app::App;
pub use builder::Builder;

pub use echo_macro::tree;

// pub use app::{App, AppState};
// pub use echo_macro::view;
// pub use ui::{Action, ActionEither, ActionFor, ActionNode, ActionOption, ActionStart, Ui};
