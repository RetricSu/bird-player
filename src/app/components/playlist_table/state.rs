use eframe::egui;

pub(crate) struct PlaylistTableState {
    drag_id: egui::Id,
    drop_id: egui::Id,
    is_dragging_id: egui::Id,
    edit_field_id: egui::Id,
    edit_track_idx_id: egui::Id,
    edit_value_id: egui::Id,
    scroll_to_idx_id: egui::Id,
    pub editing_field: Option<String>,
    pub editing_track_idx: Option<usize>,
    edit_value: Option<String>,
    pub dragged_item: Option<usize>,
    pub drop_target: Option<usize>,
    pub is_dragging: bool,
}

impl PlaylistTableState {
    pub(crate) fn load(ui: &mut egui::Ui, base_id: egui::Id) -> Self {
        let drag_id = base_id.with("drag_source");
        let drop_id = base_id.with("drop_target");
        let is_dragging_id = base_id.with("is_dragging");
        let edit_field_id = base_id.with("edit_field_id");
        let edit_track_idx_id = base_id.with("edit_track_idx_id");
        let edit_value_id = base_id.with("edit_value_id");
        let scroll_to_idx_id = base_id.with("scroll_to_idx");

        let editing_field = ui
            .memory_mut(|mem| mem.data.get_temp::<Option<String>>(edit_field_id))
            .unwrap_or(None);
        let editing_track_idx = ui
            .memory_mut(|mem| mem.data.get_temp::<Option<usize>>(edit_track_idx_id))
            .unwrap_or(None);
        let edit_value = ui.memory_mut(|mem| mem.data.get_temp::<String>(edit_value_id));
        let dragged_item = ui
            .memory_mut(|mem| mem.data.get_temp::<Option<usize>>(drag_id))
            .unwrap_or(None);
        let drop_target = ui
            .memory_mut(|mem| mem.data.get_temp::<Option<usize>>(drop_id))
            .unwrap_or(None);
        let is_dragging = ui
            .memory_mut(|mem| mem.data.get_temp::<bool>(is_dragging_id))
            .unwrap_or(false);

        Self {
            drag_id,
            drop_id,
            is_dragging_id,
            edit_field_id,
            edit_track_idx_id,
            edit_value_id,
            scroll_to_idx_id,
            editing_field,
            editing_track_idx,
            edit_value,
            dragged_item,
            drop_target,
            is_dragging,
        }
    }

    pub(crate) fn begin_drag(&mut self, ui: &mut egui::Ui, idx: usize) {
        self.dragged_item = Some(idx);
        self.is_dragging = true;
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp::<Option<usize>>(self.drag_id, Some(idx));
            mem.data.insert_temp::<bool>(self.is_dragging_id, true);
        });
    }

    pub(crate) fn clear_drag(&mut self, ui: &mut egui::Ui) {
        self.dragged_item = None;
        self.drop_target = None;
        self.is_dragging = false;
        ui.memory_mut(|mem| {
            mem.data.insert_temp::<Option<usize>>(self.drag_id, None);
            mem.data.insert_temp::<Option<usize>>(self.drop_id, None);
            mem.data.insert_temp::<bool>(self.is_dragging_id, false);
        });
    }

    pub(crate) fn set_drop_target(&mut self, ui: &mut egui::Ui, target: Option<usize>) {
        self.drop_target = target;
        ui.memory_mut(|mem| mem.data.insert_temp::<Option<usize>>(self.drop_id, target));
    }

    pub(crate) fn clear_drop_target(&mut self, ui: &mut egui::Ui) {
        self.set_drop_target(ui, None);
    }

    pub(crate) fn drop_target(&self) -> Option<usize> {
        self.drop_target
    }

    pub(crate) fn dragged_item(&self) -> Option<usize> {
        self.dragged_item
    }

    pub(crate) fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    pub(crate) fn begin_edit(
        &mut self,
        ui: &mut egui::Ui,
        field: &str,
        idx: usize,
        initial: String,
    ) {
        let field = field.to_string();
        self.editing_field = Some(field.clone());
        self.editing_track_idx = Some(idx);
        self.edit_value = Some(initial.clone());

        ui.memory_mut(|mem| {
            mem.data
                .insert_temp::<Option<String>>(self.edit_field_id, Some(field));
            mem.data
                .insert_temp::<Option<usize>>(self.edit_track_idx_id, Some(idx));
            mem.data.insert_temp::<String>(self.edit_value_id, initial);
        });
    }

    pub(crate) fn set_edit_value(&mut self, ui: &mut egui::Ui, value: String) {
        self.edit_value = Some(value.clone());
        ui.memory_mut(|mem| mem.data.insert_temp::<String>(self.edit_value_id, value));
    }

    pub(crate) fn clear_edit(&mut self, ui: &mut egui::Ui) {
        self.editing_field = None;
        self.editing_track_idx = None;
        self.edit_value = None;
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp::<Option<String>>(self.edit_field_id, None);
            mem.data
                .insert_temp::<Option<usize>>(self.edit_track_idx_id, None);
            mem.data.remove::<String>(self.edit_value_id);
        });
    }

    pub(crate) fn edit_buffer_or(&self, fallback: &str) -> String {
        self.edit_value
            .clone()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub(crate) fn take_scroll_request(&self, ui: &mut egui::Ui) -> Option<usize> {
        let request = ui.memory_mut(|mem| mem.data.get_temp::<usize>(self.scroll_to_idx_id));
        if request.is_some() {
            ui.memory_mut(|mem| mem.data.remove::<usize>(self.scroll_to_idx_id));
        }
        request
    }
}
