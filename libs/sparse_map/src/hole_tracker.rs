use core::ops::Range;

#[derive(Default)]
pub struct HoleTracker(Option<(usize, usize)>);

impl HoleTracker {
    pub(crate) fn update(&mut self, start: usize, end: usize) {
        self.0 = Some(
            self.0
                .map_or_else(|| (start, end), |(prev_start, _)| (prev_start, end)),
        );
    }
}

impl From<HoleTracker> for Option<Range<usize>> {
    fn from(value: HoleTracker) -> Self {
        value.0.map(|(start, end)| start..end)
    }
}
