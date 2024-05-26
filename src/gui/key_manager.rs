use egui::Modifiers;

use super::util::{DragOp, PainterRel};

impl PainterRel {
    pub fn key_manager(&self, keys: &[KMKey], state: &mut Option<KMKey>, ui: &egui::Ui, mut op: impl FnMut(KMKey,DragOp)) {
        // abort the key action if esc pressed
        let esc_pressed = ui.input(|i| i.key_down(egui::Key::Escape) );

        // read the current pressed key
        let mods = ui.input(|i| i.modifiers );
        
        let hov = self.hover_pos_rel();

        let is_pressed_ignmod = |v:AKey, ui: &egui::Ui|{
            match v {
                AKey::Kbd(k) => ui.input(|i| i.key_down(k) ),
                AKey::Mouse(k) => self.response.dragged_by(k) || self.response.drag_started_by(k),
            }
        };

        let is_pressed = |v:&KMKey, ui: &egui::Ui|{
            if !v.match_mods(mods) {
                return false;
            }
            is_pressed_ignmod(v.key, ui)
        };

        let first_pressed_key = keys.iter().filter(|k| is_pressed(k,ui) ).next().cloned();

        // if the base key was released, but not if modifiers change
        let mut key_was_released = state.as_ref().is_some_and(|v| !is_pressed_ignmod(v.key,ui) && !v.is_esc() );

        let mut abort_current_pressed_key = esc_pressed;

        // if suddenly pressing different key, only abort here now if outside area
        if first_pressed_key.is_some() && state.is_some() && *state != first_pressed_key {
            if hov.is_none() {
                abort_current_pressed_key = true;
            }
        }

        // abort if key release outside the area
        if first_pressed_key.is_none() && state.is_some() && hov.is_none() {
            abort_current_pressed_key = true;
        }
        if key_was_released && hov.is_none() {
            abort_current_pressed_key = true;
        }

        // do the abort
        if abort_current_pressed_key {
            if let Some(k) = state {
                op(k.clone(), DragOp::Abort);
            }
            *state = None;
            if esc_pressed {
                *state = Some(KMKey::ignmods(egui::Key::Escape));
                key_was_released = false;
            }
        }

        // if the key is just kept pressed, ignore sudden key change for now
        if !abort_current_pressed_key && first_pressed_key.is_some() && state.is_some() && !key_was_released /*&& *state == first_pressed_key*/ {
            op(state.clone().unwrap(), DragOp::Tick(hov));
            return;
        }

        // graceful key up
        if !abort_current_pressed_key && hov.is_some() && (key_was_released || (first_pressed_key.is_none() && state.is_some())) {
            op(state.clone().unwrap(), DragOp::End(hov.unwrap()));
            *state = None;
        }

        // new key press
        if first_pressed_key.is_some() && state.is_none() && hov.is_some() && !esc_pressed {
            *state = first_pressed_key.clone();
            op(first_pressed_key.unwrap(), DragOp::Start(hov.unwrap()));
            return;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AKey {
    Kbd(egui::Key),
    Mouse(egui::PointerButton),
}

impl From<egui::Key> for AKey {
    fn from(value: egui::Key) -> Self {
        Self::Kbd(value)
    }
}

impl From<egui::PointerButton> for AKey {
    fn from(value: egui::PointerButton) -> Self {
        Self::Mouse(value)
    }
}

#[derive(Clone, Debug)]
pub struct KMKey {
    pub(crate) key: AKey,
    pub(crate) mods: Modifiers,
    pub(crate) modmask: Modifiers,
}

impl KMKey {
    pub fn nomods(key: impl Into<AKey>) -> Self {
        Self {
            key: key.into(),
            mods: Modifiers::NONE,
            modmask: ALL_MODS,
        }
    }

    pub fn ignmods(key: impl Into<AKey>) -> Self {
        Self {
            key: key.into(),
            mods: Modifiers::NONE,
            modmask: Modifiers::NONE,
        }
    }

    pub fn with_ctrl(key: impl Into<AKey>, ctrl: bool) -> Self {
        Self {
            key: key.into(),
            mods: Modifiers { ctrl, ..Default::default() },
            modmask: Modifiers::CTRL,
        }
    }

    fn match_mods(&self, other: Modifiers) -> bool {
        mask_mods(self.mods, self.modmask) == mask_mods(other, self.modmask)
    }

    fn stated(&self) -> (AKey,Modifiers) {
        (self.key,mask_mods(self.mods, self.modmask))
    }

    fn is_esc(&self) -> bool {
        matches!(self.key, AKey::Kbd(egui::Key::Escape))
    }
}

impl PartialEq<(AKey,Modifiers)> for KMKey {
    fn eq(&self, other: &(AKey,Modifiers)) -> bool {
        self.key == other.0 && self.match_mods(other.1)
    }
}

// impl PartialEq<KMKey> for (AKey,Modifiers) {
//     fn eq(&self, other: &KMKey) -> bool {
//         self.0 == other.key && other.match_mods(self.1)
//     }
// }

impl PartialEq for KMKey {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && mask_mods(self.mods, self.modmask) == mask_mods(other.mods, other.modmask)
    }
}

const ALL_MODS: Modifiers = Modifiers {
    alt: true,
    ctrl: true,
    shift: true,
    mac_cmd: true,
    command: true,
};

fn mask_mods(mods: Modifiers, mask: Modifiers) -> Modifiers {
    let mut emter = Modifiers::NONE;
    emter.alt     = mods.alt     & mask.alt;
    emter.ctrl    = mods.ctrl    & mask.ctrl;
    emter.shift   = mods.shift   & mask.shift;
    emter.mac_cmd = mods.mac_cmd & mask.mac_cmd;
    emter.command = mods.command & mask.command;
    emter
}
