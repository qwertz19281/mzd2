use serde::{Deserialize, Serialize};

pub struct Tags {

}

#[derive(Deserialize,Serialize)]
pub struct TagState {
    pos: [u32;2],
    title: String,
    show_always: bool,
    text: String,
    //warp: Option<WarpDest>,
}
