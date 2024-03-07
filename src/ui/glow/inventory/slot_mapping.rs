use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::ui::glow::{render::{hud::Hud, inventory::InventoryWindow, Renderer}, ui::{Container, VAttach}};

use super::{Item, Slot};

/// A mapping of each slot and it's position on the screen. This makes the job
/// of rendering and locating slots easier for inventories. This class only
/// expects positions in pixels relative to the top-left corner of the window.
pub struct SlotMapping {
    /// A mapping of slot_id to (position, slot). The position is the distance
    /// between the window's top-left corner, and the slot's top-left corner in
    /// pixels. This doesn't include the border frame around each slot, just
    /// the 16x16 content. This uses a HashMap since the slot ids may have gaps
    /// where the child slot mapping should be used instead.
    slots: HashMap<u16, ((i32, i32), Slot)>,
    /// The size of the window in pixels.
    size: (i32, i32),
    dirty: bool,
    /// A child slot mapping to be used as a fallback if a slot wasn't found
    /// in the parent mapping. This is used to handle the base inventory.
    child: Option<ChildSlotMapping>,
}

/// A container required to store information about how a child slot mapping
/// behaves under a specific parent mapping.
struct ChildSlotMapping {
    /// The child's slots.
    pub slots: Arc<RwLock<SlotMapping>>,
    /// The offset between the top-left corner of the current mapping window,
    /// and the corner of the sub-section that the child mapping manages, in
    /// pixels.
    pub offset: (i32, i32),
    /// The slot ids to be used on the parent interface to access slots in the
    /// child. We use the index of each parent slot id as the child slot id.
    /// For example, if set to `[5, 6, 7]`, then accessing slot id `6` on the
    /// parent will look up the slot_id `1` on the child.
    pub range: Vec<u16>,
}

impl SlotMapping {
    /// Create a new slot mapping for a window with the given size in pixels.
    pub fn new(size: (i32, i32)) -> Self {
        Self {
            slots: HashMap::new(),
            size,
            dirty: false,
            child: None,
        }
    }

    /// Get the number of slots in this mapping, including any child slots.
    pub fn size(&self) -> u16 {
        match &self.child {
            Some(child) => self.slots.len() as u16 + child.slots.read().size(),
            None => self.slots.len() as u16,
        }
    }

    /// Set the child slot mapping. This will replace any child previously set.
    pub fn set_child(
        &mut self,
        slots: Arc<RwLock<SlotMapping>>,
        offset: (i32, i32),
        range: Vec<u16>,
    ) {
        self.child = Some(ChildSlotMapping {
            slots,
            offset,
            range,
        });
    }

    /// Add a slot to this mapping with a specific pixel offset where the slot
    /// will be located.
    pub fn add_slot(&mut self, slot_id: u16, offset: (i32, i32)) {
        self.slots
            .insert(slot_id, (offset, Slot::new(0.0, 0.0, 0.0)));
    }

    /// Get the item contained in a slot, if any.
    pub fn get_item(&self, slot_id: u16) -> Option<Item> {
        if let Some((_, slot)) = self.slots.get(&slot_id) {
            return slot.item.clone();
        }

        if let Some(child) = &self.child {
            let slot_id = child.range.iter().position(|id| *id == slot_id).unwrap();
            return child.slots.read().get_item(slot_id as u16);
        }

        None
    }

    /// Store an item in a slot. This will replace any previous value for that
    /// slot.
    pub fn set_item(&mut self, slot_id: u16, item: Option<Item>) {
        if let Some((_, slot)) = self.slots.get_mut(&slot_id) {
            slot.item = item;
            self.dirty = true;
            return;
        }

        if let Some(child) = &self.child {
            let slot_id = child.range.iter().position(|id| *id == slot_id).unwrap();
            child.slots.write().set_item(slot_id as u16, item);
        }
    }

    /// Get a slot in a specific location on the screen.
    pub fn get_slot(&self, x: f64, y: f64) -> Option<u16> {
        for (i, (_, slot)) in &self.slots {
            if slot.is_within(x, y) {
                return Some(*i);
            }
        }

        if let Some(child) = &self.child {
            return child
                .slots
                .read()
                .get_slot(x, y)
                .map(|i| child.range[i as usize]);
        }

        None
    }

    /// Reposition all slots so that they can be rendered in the correct
    /// location.
    pub fn update_icons(
        &mut self,
        renderer: &Arc<Renderer>,
        offset: (i32, i32),
        outer_size: Option<(i32, i32)>,
    ) {
        let scale = Hud::icon_scale(renderer) as i32;
        let slot_size = scale as f64 * 16.0;
        let center = renderer.screen_data.read().center();
        let size = match outer_size {
            Some((x, y)) => (x, y),
            None => (self.size.0, self.size.1),
        };

        let x_offset = center.0 as i32 - size.0 * scale / 2 + offset.0 * scale;
        let y_offset = center.1 as i32 - size.1 * scale / 2 + offset.1 * scale;

        for (_, ((slot_x, slot_y), slot)) in &mut self.slots.iter_mut() {
            let x = x_offset + *slot_x * scale;
            let y = y_offset + *slot_y * scale;
            slot.update_position(x as f64, y as f64, slot_size);
        }

        if let Some(child) = &self.child {
            child
                .slots
                .write()
                .update_icons(renderer, child.offset, Some(self.size));
        }

        self.dirty = true;
    }

    /// Render slots if required.
    pub fn tick(
        &mut self,
        renderer: &Arc<Renderer>,
        ui_container: &mut Container,
        inventory_window: &mut InventoryWindow,
        element_idx: usize,
    ) {
        if self.dirty {
            self.dirty = false;
            inventory_window
                .elements
                .get_mut(element_idx)
                .unwrap()
                .clear();
            inventory_window
                .text_elements
                .get_mut(element_idx)
                .unwrap()
                .clear();
            if let Some(child) = &self.child {
                child.slots.write().dirty = true;
            }
            for (_, slot) in self.slots.values() {
                if let Some(item) = &slot.item {
                    inventory_window.draw_item_internally(
                        item,
                        slot.x,
                        slot.y,
                        element_idx,
                        ui_container,
                        renderer,
                        VAttach::Top,
                    );
                }
            }
        }

        if let Some(child) = &self.child {
            child
                .slots
                .write()
                .tick(renderer, ui_container, inventory_window, element_idx + 1);
        }
    }
}
