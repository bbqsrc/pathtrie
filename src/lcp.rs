#[inline]
pub(crate) fn find_common_prefix(source: &[u8], candidate: &[u8]) -> Prefix {
    let result = source
        .iter()
        .zip(candidate.iter())
        .position(|(x, y)| x != y);

    match (source.len() == candidate.len(), result) {
        (_, Some(0)) => Prefix::NoMatch(source.cmp(candidate)),
        (_, Some(len)) => Prefix::Divergent(len),
        (false, None) => {
            if source.len() > candidate.len() {
                Prefix::PerfectSubset(candidate.len())
            } else {
                Prefix::Incomplete(source.len())
            }
        }
        (true, None) => Prefix::Exact,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Prefix {
    NoMatch(std::cmp::Ordering), // No shared prefix at all, eg "abc" and "cdefg"
    PerfectSubset(usize),        // eg "foo" inside "foobar"
    Divergent(usize),            // eg "foobar" and "foojam"
    Incomplete(usize),           // eg provided key "abcdef" but source key is "abc"
    Exact,                       // eg "abc" and "abc"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn no_match() {
        assert_eq!(
            find_common_prefix("abc".as_bytes(), "def".as_bytes()),
            Prefix::NoMatch(Ordering::Less)
        );

        assert_eq!(
            find_common_prefix("DEF".as_bytes(), "ABC".as_bytes()),
            Prefix::NoMatch(Ordering::Greater)
        );

        assert_eq!(
            find_common_prefix(" DEF".as_bytes(), "DEF".as_bytes()),
            Prefix::NoMatch(Ordering::Less)
        );
    }

    #[test]
    fn perfect_subset() {
        assert_eq!(
            find_common_prefix("abcdef".as_bytes(), "abc".as_bytes()),
            Prefix::PerfectSubset(3)
        );

        assert_eq!(
            find_common_prefix("longer/test/item".as_bytes(), "longer/test".as_bytes()),
            Prefix::PerfectSubset(11)
        );
    }

    #[test]
    fn divergent() {
        assert_eq!(
            find_common_prefix("a/b/c".as_bytes(), "a/b/d".as_bytes()),
            Prefix::Divergent(4)
        );
    }

    #[test]
    fn incomplete() {
        assert_eq!(
            find_common_prefix("a/b/c".as_bytes(), "a/b/c/d".as_bytes()),
            Prefix::Incomplete(5)
        );
    }

    #[test]
    fn exact() {
        assert_eq!(
            find_common_prefix("a/b/c".as_bytes(), "a/b/c".as_bytes()),
            Prefix::Exact
        );
    }
}
