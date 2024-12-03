use core::ops::Bound;
use headers::Range as RangeHeader;

/// HTTP Range header (single range only)
///
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Range
///
/// Closed interval. Lower and upper bounds are both included.
#[derive(Clone, Default)]
pub struct Range(pub Option<usize>, pub Option<usize>);

impl TryFrom<&Range> for String {
    type Error = ();

    fn try_from(value: &Range) -> Result<Self, Self::Error> {
        match value {
            Range(None, None) => Err(()),
            Range(Some(start), None) => Ok(format!("bytes={start}-")),
            Range(None, Some(end)) => Ok(format!("bytes=-{end}")),
            Range(Some(start), Some(end)) => Ok(format!("bytes={start}-{end}")),
        }
    }
}

impl TryFrom<core::ops::Range<usize>> for Range {
    type Error = ();

    fn try_from(range: core::ops::Range<usize>) -> Result<Self, Self::Error> {
        if range.end > 0 {
            return Ok(Self(Some(range.start), Some(range.end - 1)));
        }

        Err(())
    }
}

impl TryFrom<&RangeHeader> for Range {
    type Error = ();

    fn try_from(value: &RangeHeader) -> Result<Self, Self::Error> {
        let range = {
            let mut iter = value.satisfiable_ranges(u64::MAX);
            let range = iter.next();

            // There must be only one range specified.
            if range.is_some() && iter.next().is_some() {
                return Err(());
            }

            range
        };

        let (lower, upper) = match range {
            None => return Err(()),
            Some(range) => range,
        };

        Ok(Self(convert_bound(lower, 0), convert_bound(upper, 1)))
    }
}

impl Range {
    pub fn as_range(&self, len: usize) -> Option<core::ops::Range<usize>> {
        if self.0.is_none() && self.1.is_none() {
            return None;
        }

        let lower = self.0.unwrap_or(0).min(len);
        let upper = self.1.map(|i| i + 1).unwrap_or(len).min(len);

        Some(lower..upper)
    }

    pub fn is_empty(&self) -> bool {
        match (self.0, self.1) {
            (Some(x), Some(y)) => x > y,
            _ => false,
        }
    }

    pub fn add_start(&mut self, len: usize) {
        *self.0.get_or_insert_default() += len;
    }
}

fn convert_bound(bound: Bound<u64>, addend: u64) -> Option<usize> {
    match bound {
        Bound::Included(v) | Bound::Excluded(v) => (v + addend).try_into().ok(),
        _ => None,
    }
}
