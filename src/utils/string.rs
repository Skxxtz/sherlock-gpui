#[allow(unused)]
pub trait TrimInPlace {
    fn trim_in_place(&mut self);
}

impl TrimInPlace for String {
    fn trim_in_place(&mut self) {
        let end = self.trim_end().len();
        self.truncate(end);
        let start = self.len() - self.trim_start().len();
        self.drain(..start);
    }
}
