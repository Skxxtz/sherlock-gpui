use crate::ui::model::ItemIdx;

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    /// The unique index of the item
    pub data_idx: ItemIdx,

    /// Whether the current item is selected by the user
    pub is_selected: bool,
}

impl Selection {
    #[inline(always)]
    pub fn new(data_idx: ItemIdx, is_selected: bool) -> Self {
        Self {
            data_idx,
            is_selected,
        }
    }
}
