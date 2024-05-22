use egui_dock::{DockArea, DockState, Node, NodeIndex, Split, SurfaceIndex, TabViewer, Tree};

use crate::util::{MapId, TilesetId};

use super::init::SharedApp;
use super::palette::{lru_ui, palette_ui};

pub struct Docky {
    state: Option<DockState<DockTab>>,
    last_focused_map: Option<MapId>,
    last_focused_tileset: Option<TilesetId>,
    last_rendered_map: Option<MapId>,
    last_rendered_tileset: Option<TilesetId>,
    pub remove_tabs: Vec<DockTab>,
    pub add_tabs: Vec<DockTab>,
}

impl Docky {
    pub fn new() -> Self {
        Self {
            state: Some(create_initial()),
            last_focused_map: None,
            last_focused_tileset: None,
            last_rendered_map: None,
            last_rendered_tileset: None,
            remove_tabs: vec![],
            add_tabs: vec![],
        }
    }
}

fn create_initial() -> DockState<DockTab> {
    let mut state = DockState::new(vec![DockTab::Draw]);
    let surf = state.main_surface_mut();
    let [orig,_] = surf.split_left(NodeIndex::root(), 0.5, vec![DockTab::Palette]);
    surf.split_right(orig, 0.9, vec![DockTab::Lru]);
    //surf.split_above(left, 0.01, vec![]);
    state
}

#[derive(Clone, PartialEq)]
pub enum DockTab {
    Map(MapId),
    Tileset(TilesetId),
    Palette,
    Lru,
    Draw,
}

impl DockTab {
    fn same_type(&self, other: &Self) -> bool {
        match self {
            DockTab::Map(_) => matches!(other,Self::Map(_)),
            DockTab::Tileset(_) => matches!(other,Self::Tileset(_)),
            DockTab::Palette => matches!(other,Self::Palette),
            DockTab::Lru => matches!(other,Self::Lru),
            DockTab::Draw => matches!(other,Self::Draw),
        }
    }
}

impl SharedApp {
    pub fn dock_ui(&mut self, ui: &mut egui::Ui) {
        self.dock_op();
        let mut state = self.dock.state.take().unwrap();
        let mut viewer = TabV(self);
        DockArea::new(&mut state)
            .style(egui_dock::Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut viewer);
        self.dock.state = Some(state);
        self.dock_op();
    }

    fn probe_active(&mut self) {
        if let Some((_,tab)) = self.dock.state.as_mut().unwrap().find_active_focused() {
            match tab {
                DockTab::Map(id) => self.dock.last_focused_map = Some(*id),
                DockTab::Tileset(id) => self.dock.last_focused_tileset = Some(*id),
                _ => {}
            }
        }
    }

    fn dock_op(&mut self) {
        let state = self.dock.state.as_mut().unwrap();
        for rm in self.dock.remove_tabs.drain(..) {
            if let Some((a,b,c)) = state.find_tab(&rm) {
                match rm {
                    DockTab::Map(_) | DockTab::Tileset(_) => {
                        state.remove_tab((a,b,c));
                    },
                    _ => {},
                }
            }
        }
        if let Some(v) = self.sam.set_focus_to.take() {
            if let Some((a,b,c)) = state.find_tab(&v) {
                state.set_active_tab((a,b,c));
                state.set_focused_node_and_surface((a,b));
            }
        }
        self.probe_active();
        let state = self.dock.state.as_mut().unwrap();

        fn try_append_tab_to_node(state: &mut DockState<DockTab>, si: SurfaceIndex, ni: NodeIndex, tab: &DockTab) -> bool {
            if let Some(tree) = state.get_surface_mut(si).and_then(|s| s.node_tree_mut() ) {
                let node = &mut tree[ni];
                node.append_tab(tab.clone());
                return true;
            }
            false
        }

        // fn try_add_split(state: &mut DockState<DockTab>, si: SurfaceIndex, ni: NodeIndex, split: Split, frac: f32, tab: &DockTab) -> bool {
        //     if let Some(tree) = state.get_surface_mut(si).and_then(|s| s.node_tree_mut() ) {
        //         tree.split(ni, split, frac, Node::leaf_with(vec![tab.clone()]));
        //         return true;
        //     }
        //     false
        // }

        fn find_tab_scored(a: &Tree<DockTab>, node: NodeIndex, b: &mut impl FnMut(&DockTab)->u8) -> u8 {
            let mut score = 0;
            match &a[node] {
                Node::Empty => {},
                Node::Leaf { tabs, .. } => for t in tabs {
                    score = score.max(b(t));
                },
                Node::Vertical {..} | Node::Horizontal {..} => {
                    score = score.max(find_tab_scored(a, node.left(), b));
                    score = score.max(find_tab_scored(a, node.right(), b));
                },
            }
            score
        }

        for add in self.dock.add_tabs.drain(..) {
            match add {
                DockTab::Map(_) => {'tri:{
                    if let Some((a,b,_)) = self.dock.last_focused_map.and_then(|b| state.find_tab(&DockTab::Map(b)) ) {
                        if try_append_tab_to_node(state, a, b, &add) {
                            break 'tri;
                        }
                    }
                    if let Some((a,b,_)) = self.dock.last_rendered_map.and_then(|b| state.find_tab(&DockTab::Map(b)) ) {
                        if try_append_tab_to_node(state, a, b, &add) {
                            break 'tri;
                        }
                    }
                    let (a,b,_) = state.find_tab(&DockTab::Palette).unwrap();
                    state.split((a,b), Split::Above, 0.9, Node::leaf_with(vec![add]));
                }},
                DockTab::Tileset(_) => {'tri:{
                    if let Some((a,b,_)) = self.dock.last_focused_tileset.and_then(|b| state.find_tab(&DockTab::Tileset(b)) ) {
                        if try_append_tab_to_node(state, a, b, &add) {
                            break 'tri;
                        }
                    }
                    if let Some((a,b,_)) = self.dock.last_rendered_tileset.and_then(|b| state.find_tab(&DockTab::Tileset(b)) ) {
                        if try_append_tab_to_node(state, a, b, &add) {
                            break 'tri;
                        }
                    }
                    let (a,mut b,_) = state.find_tab(&DockTab::Draw).unwrap();
                    while let Some(bp) = b.parent() {
                        if let Some(tree) = state.get_surface_mut(a).and_then(|s| s.node_tree_mut() ) {
                            if matches!(&tree[bp], Node::Horizontal{..} | Node::Vertical{..}) {
                                let what_tab = |node| find_tab_scored(tree, node, &mut |t|
                                    match t {
                                        DockTab::Palette => 1,
                                        DockTab::Lru => 2,
                                        DockTab::Draw => 3,
                                        DockTab::Map(_) => 4,
                                        _ => 0,
                                    }
                                );
                                let (l,r) = (what_tab(bp.left()),what_tab(bp.right()));
                                if matches!(&tree[bp],Node::Horizontal{..}) && l == 1 {
                                    break;
                                }
                                if l != 0 && r != 0 && l <= 3 && r <= 3 && l != r {
                                    b = bp;
                                    continue;
                                }
                            }
                        }
                        break;
                    }
                    state.split((a,b), Split::Below, 0.55, Node::leaf_with(vec![add]));
                }},
                _ => {},
            }
        }
        self.probe_active();
    }
}

struct TabV<'a>(&'a mut SharedApp);

impl TabViewer for TabV<'_> {
    type Tab = DockTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            DockTab::Map(id) => {
                if let Some(map) = self.0.maps.open_maps.get(&*id) {
                    let map = map.borrow();
                    format!("Map - {}", map.state.title).into()
                } else {
                    "".into()
                }
            },
            DockTab::Tileset(id) => {
                if let Some(tileset) = self.0.tilesets.open_tilesets.get(&*id) {
                    (&tileset.state.title).into()
                } else {
                    "".into()
                }
            },
            DockTab::Palette => "Palette".into(),
            DockTab::Lru => "LastUsed".into(),
            DockTab::Draw => "Draw".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            DockTab::Map(id) => if let Some(map) = self.0.maps.open_maps.get(&*id) {
                let mut map = map.borrow_mut();
                self.0.dock.last_rendered_map = Some(*id);
                map.ui_map(
                    &mut self.0.palette,
                    ui,
                    &mut self.0.sam,
                    &self.0.maps,
                );
            },
            DockTab::Tileset(id) => if let Some(tileset) = self.0.tilesets.open_tilesets.get_mut(&*id) {
                self.0.dock.last_rendered_tileset = Some(*id);
                tileset.ui(
                    &mut self.0.palette,
                    ui,
                    &mut self.0.sam,
                );
            },
            DockTab::Palette => palette_ui(self.0, ui),
            DockTab::Lru => lru_ui(self.0, ui),
            DockTab::Draw => if let Some(map) = self.0.dock.last_focused_map.and_then(|id| self.0.maps.open_maps.get_mut(&id)) {
                let mut map = map.borrow_mut();
                map.ui_draw(
                    &mut self.0.palette,
                    ui,
                    &mut self.0.sam,
                );
            },
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            DockTab::Map(id) => id.i_map,
            DockTab::Tileset(id) => id.i,
            DockTab::Palette => egui::Id::new("Palette"),
            DockTab::Lru => egui::Id::new("LruPalette"),
            DockTab::Draw => egui::Id::new("Draw"),
        }
    }

    fn closeable(&mut self, _: &mut Self::Tab) -> bool {
        false
    }

    fn force_close(&mut self, tab: &mut Self::Tab) -> bool {
        match tab {
            DockTab::Map(id) => !self.0.maps.open_maps.contains_key(id),
            DockTab::Tileset(id) => !self.0.tilesets.open_tilesets.contains_key(id),
            _ => false,
        }
    }

    fn allowed_in_windows(&self, _: &mut Self::Tab) -> bool {
        false
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        match tab {
            DockTab::Lru => [false,true],
            _ => [false,false]
        }
    }
}
