#[macro_use]
pub mod util;
pub mod gui;
pub mod map;
pub mod cli;
pub mod convert_0_1;

type SRc<T> = std::rc::Rc<T>;
