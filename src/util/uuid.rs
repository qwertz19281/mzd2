use std::path::{Path, PathBuf};

use egui::ahash::HashMap;
use uuid::Uuid;

use crate::gui::map::RoomId;
use crate::util::{seltrix_resource_path, tex_resource_path};

use super::MapId;

pub enum UUIDTarget {
    Map(MapId),
    Room(MapId,RoomId),
    Resource(MapId,RoomId),
}

pub type UUIDMap = HashMap<Uuid,UUIDTarget>;

pub fn generate_uuid(check: &UUIDMap) -> Uuid {
    loop {
        let uuid = Uuid::now_v7();
        if check.contains_key(&uuid) {
            continue;
        }
        return uuid;
    }
}

pub fn generate_res_uuid(check: &UUIDMap, map_path: impl Into<PathBuf>) -> Uuid {
    let map_path = map_path.into();
    loop {
        let uuid = generate_uuid(check);
        fn check_exist(path: &Path) -> bool {
            path.symlink_metadata().is_ok()
        }
        let tex_path = tex_resource_path(&map_path, &uuid);
        let seltrix_path = seltrix_resource_path(&map_path, &uuid);
        if check_exist(&tex_path) || check_exist(&seltrix_path) {
            continue;
        }
        return uuid;
    }
}
