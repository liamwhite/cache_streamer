use core::ops::Range;

/// Returns (`ab` intersects `cd` && `ab` starts before `cd`).
pub(crate) fn lt_intersecting<I: PartialOrd>(ab: &Range<I>, cd: &Range<I>) -> bool {
    ab.start < cd.start && cd.start < ab.end
}

/// Returns (`ab` intersects `cd` && `ab` starts at or after `cd`).
pub(crate) fn gte_intersecting<I: PartialOrd>(ab: &Range<I>, cd: &Range<I>) -> bool {
    ab.start < cd.end && cd.start <= ab.start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lt_intersecting() {
        // ab less than cd and not intersecting
        assert_eq!(lt_intersecting(&(0..1), &(1..2)), false);
        assert_eq!(lt_intersecting(&(0..1), &(2..3)), false);

        // ab less than cd and intersecting
        assert_eq!(lt_intersecting(&(0..2), &(1..3)), true);

        // ab less than but completely contains cd
        assert_eq!(lt_intersecting(&(0..4), &(1..3)), true);

        // ab not less than cd
        assert_eq!(lt_intersecting(&(0..1), &(0..2)), false);

        // ab greater than cd
        assert_eq!(lt_intersecting(&(1..2), &(0..1)), false);
    }

    #[test]
    fn test_gte_intersection() {
        // ab less than cd and not intersecting
        assert_eq!(gte_intersecting(&(0..1), &(1..2)), false);
        assert_eq!(gte_intersecting(&(0..1), &(2..3)), false);

        // ab less than cd and intersecting
        assert_eq!(gte_intersecting(&(0..2), &(1..3)), false);

        // ab equal to cd
        assert_eq!(gte_intersecting(&(0..1), &(0..1)), true);

        // ab less than but completely contains cd
        assert_eq!(gte_intersecting(&(0..4), &(1..3)), false);

        // ab equal to and completely contains cd
        assert_eq!(gte_intersecting(&(0..4), &(0..3)), true);

        // ab greater than cd and intersecting
        assert_eq!(gte_intersecting(&(2..3), &(1..3)), true);

        // ab greater than cd
        assert_eq!(gte_intersecting(&(2..3), &(1..2)), false);
    }
}
