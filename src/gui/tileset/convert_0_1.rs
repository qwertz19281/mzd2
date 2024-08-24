
use std::path::Path;

use serde::Deserialize;

use crate::convert_0_1::{OldSelMatrix, OldSelMatrixLayered};
use crate::gui::draw_state::DrawMode;
use crate::gui::dsel_state::DSelMode;
use crate::gui::sel_matrix::SelMatrix;

use super::TilesetState;

#[derive(Deserialize)]
pub struct OldTilesetState {
    pub title: String,
    pub zoom: u32,
    pub voff: [f32;2],
    pub validate_size: [u32;2],
    pub sel_matrix: OldSelMatrix,
    pub draw_draw_mode: DrawMode,
    pub draw_sel: DSelMode,
    pub ds_replace: bool,
    pub dsel_whole: bool,
}

pub(super) fn try_convert_tileset(epath: &Path, tpath: &Path) -> anyhow::Result<(TilesetState,SelMatrix)> {
    let data = std::fs::read(epath)?;
    let old_state = serde_json::from_slice::<OldTilesetState>(&data)?;

    let sel_matrix_dims = old_state.sel_matrix.dims;
    let old_sml = OldSelMatrixLayered { dims: sel_matrix_dims, layers: vec![old_state.sel_matrix] };
    let mut new_sml = old_sml.convert_to_new();

    let new_state = TilesetState {
        mzd_format: 2,
        json_ident: None,
        title: old_state.title,
        zoom: old_state.zoom,
        voff: old_state.voff,
        validate_size: old_state.validate_size,
        draw_draw_mode: old_state.draw_draw_mode,
        draw_sel: old_state.draw_sel,
        ds_replace: old_state.ds_replace,
        dsel_whole: old_state.dsel_whole,
    };

    Ok((new_state,new_sml.layers.swap_remove(0)))
}
