use std::{error::Error, fmt::Display, ops::RangeBounds};

/// Represents byte index in the input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[cfg(feature = "miette")]
impl From<Span> for miette::SourceSpan {
    fn from(value: Span) -> Self {
        Self::from(value.start..value.end)
    }
}

#[derive(Debug)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}

impl<T: Error> Error for Spanned<T> {}

impl<T: Error> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Self { inner, span }
    }
}

impl<T: RangeBounds<usize>> From<T> for Span {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl Span {
    pub fn new(range: impl RangeBounds<usize>) -> Self {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&v) => v,
            std::ops::Bound::Excluded(&v) => v.saturating_sub(1),
            std::ops::Bound::Unbounded => panic!("encountered span with unbounded start range"),
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(&v) => v.saturating_add(1),
            std::ops::Bound::Excluded(&v) => v,
            std::ops::Bound::Unbounded => panic!("encountered span with unbounded end range"),
        };
        Self { start, end }
    }

    /// Combine 2 spans into one
    pub fn merge(&self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn unbounded_ranges_panic() {
        Span::new(..);
    }

    #[test]
    #[should_panic]
    fn unbounded_end_ranges_panic() {
        Span::new(0..);
    }

    #[test]
    #[should_panic]
    fn unbounded_start_ranges_panic() {
        Span::new(..0);
    }
}
