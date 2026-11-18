#[derive(Clone, Copy, Debug)]
pub struct Selection(bool);

impl Selection {
    #[inline(always)]
    pub fn new(is_selected: bool) -> Self {
        Self(is_selected)
    }
    #[inline(always)]
    pub fn is(&self) -> bool {
        self.0
    }
}

impl std::ops::Not for Selection {
    type Output = bool;
    fn not(self) -> Self::Output {
        !self.0
    }
}
