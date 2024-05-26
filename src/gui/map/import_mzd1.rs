use std::path::PathBuf;

use anyhow::{ensure, anyhow, bail};
use image::GenericImageView;
use regex::Regex;

use crate::gui::map::next_ur_op_id;
use crate::gui::map::room_ops::{RoomOp, try_6_sides};
use crate::gui::sel_matrix::{SelEntryWrite, SelEntry};
use crate::gui::util::{ArrUtl, RfdUtil};
use crate::util::{gui_error, next_op_gen_evo};

use super::uuid::UUIDMap;
use super::Map;

impl Map {
    pub(super) fn ui_import_mzd1(&mut self, uuidmap: &mut UUIDMap) -> bool {
        let Some(ssel_coord) = self.state.ssel_coord else {return false};

        let dialog = rfd::FileDialog::new();
        let result = dialog
            .set_title("Import mzd1")
            .try_set_parent()
            .pick_folder();
        
        let Some(path) = result else {return false};

        if let Err(e) = self.import_mzd1(ssel_coord, path, uuidmap) {
            gui_error("Failed to import mzd1", e);
            false
        } else {
            true
        }
    }

    pub fn import_mzd1(&mut self, dest: [u8;3], mut level_dir: PathBuf, uuidmap: &mut UUIDMap) -> anyhow::Result<()> {
        ensure!(
            self.state.rooms_size[0] >= 160 && self.state.rooms_size[1] >= 128,
            "map rooms_size not large enough to hold mzd1 rooms (160x128)"
        );

        let lvl_sub_folder = {
            let mut p = level_dir.clone();
            p.push("level");
            p
        };

        if lvl_sub_folder.is_dir() {
            level_dir = lvl_sub_folder;
        }

        let coord_regex = Regex::new(r"(?m)(-?[0-9]+) (-?[0-9]+)-(-?[0-9]+)").unwrap();

        let mut rooms = vec![];

        for f in std::fs::read_dir(&level_dir)? {
            let f = f?;
            let name = f.file_name();
            let name = name.to_string_lossy();
            if !name.ends_with(".png") {continue}

            let matches = coord_regex.captures(name.as_ref()).ok_or(anyhow!("name coord regex no matches"))?;
            ensure!(matches.len() == 4, "not 3 coords in name");

            dbg!(&matches);

            let z = matches.get(1).unwrap().as_str().parse::<i32>()?;
            let x = matches.get(2).unwrap().as_str().parse::<i32>()?;
            let y = matches.get(3).unwrap().as_str().parse::<i32>()?;

            let dest: [u8;3] = [
                dbg!(dest[0] as i32 + x).try_into()?,
                dbg!(dest[1] as i32 + y).try_into()?,
                dbg!(dest[2] as i32 + z).try_into()?,
            ];

            if self.room_matrix.get(dest).is_some() {
                bail!("room overlap")
            }

            let file_content = std::fs::read(f.path())?;
            let image = image::load_from_memory(&file_content)?;
            ensure!(image.dimensions() == (160,128), "mzd1 image file with wrong dims");

            rooms.push((dest,image));
        }

        if rooms.is_empty() {return Ok(());}

        rooms.sort_by_key(|(c,_)| *c );
        rooms.dedup_by_key(|(c,_)| *c );

        let op_evo = next_op_gen_evo();
        self.latest_used_opevo = op_evo;

        let mut undo_ops = vec![];

        for (coord,image) in &rooms {
            let roomcreate_op = self.create_create_room(*coord, uuidmap).unwrap();
            let ur = self.apply_room_op(roomcreate_op, uuidmap);
            let room_id = match &ur {
                &super::room_ops::RoomOp::Del(id) => id,
                _ => panic!(),
            };
            undo_ops.push(ur);

            let overlay_pos = self.state.rooms_size.sub([160,128]).div([2,2]).div8().mul8();

            let room = self.state.rooms.get_mut(room_id).unwrap();

            image::imageops::replace(
                &mut room.loaded.as_mut().unwrap().image.img,
                image,
                overlay_pos[0] as i64,
                overlay_pos[1] as i64,
            );
            
            for y in overlay_pos[1] .. overlay_pos[1] + 128 {
                for x in overlay_pos[0] .. overlay_pos[0] + 160 {
                    *room.loaded.as_mut().unwrap().sel_matrix.layers[0].get_mut([x/8,y/8]).unwrap() = SelEntry { start: [0,0], size: [1,1] };
                }
            }

            room.op_evo = op_evo;
        }

        for (coord,_) in &rooms {
            try_6_sides(*coord, |c,ax,dir| {
                if let Some(room) = self.room_matrix.get(c).and_then(|&r| self.state.rooms.get_mut(r) ) {
                    let set_conn = room.op_evo == op_evo;
                    room.dirconn[ax as usize][!dir as usize] = set_conn;
                    if let Some(room) = self.room_matrix.get(*coord).and_then(|&r| self.state.rooms.get_mut(r) ) {
                        room.dirconn[ax as usize][dir as usize] = set_conn;
                    }
                }
            });
        }

        if undo_ops.is_empty() {return Ok(());}

        self.undo_buf.push_back((RoomOp::Multi(undo_ops),next_ur_op_id()));

        self.after_room_op_apply_invalidation(false);

        Ok(())
    }
}
