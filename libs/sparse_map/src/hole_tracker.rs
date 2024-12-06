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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let tracker = HoleTracker::default();
        let result: Option<Range<usize>> = tracker.into();

        assert_eq!(result, None);
    }

    #[test]
    fn test_single_range() {
        let mut tracker = HoleTracker::default();
        tracker.update(0, 1);

        let result: Option<Range<usize>> = tracker.into();

        assert_eq!(result, Some(0..1));
    }

    #[test]
    fn test_multiple_ranges() {
        let mut tracker = HoleTracker::default();
        tracker.update(0, 1);
        tracker.update(2, 3);

        let result: Option<Range<usize>> = tracker.into();

        assert_eq!(result, Some(0..3));
    }
}
