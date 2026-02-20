use crate::{PromiseDAG, PromiseSingle};

/// DFS iterator over DAG nodes that emits single promises
#[derive(Debug, Clone)]
pub struct IntoIter {
    stack: Vec<PromiseDAG>,
}

impl IntoIter {
    fn new(d: PromiseDAG) -> Self {
        Self { stack: vec![d] }
    }
}

impl IntoIterator for PromiseDAG {
    type Item = PromiseSingle;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}

impl Iterator for IntoIter {
    type Item = PromiseSingle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let d = self.stack.last_mut()?;

            if let Some(p) = d.then.pop() {
                return Some(p);
            }

            if let Some(after) = d.after.pop() {
                self.stack.push(after);
                continue;
            }

            let d = self.stack.pop();
            debug_assert!(d.as_ref().is_some_and(PromiseDAG::is_empty));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.stack.last().map_or(0, |d| d.then.len()), None)
    }
}

/// DFS iterator over DAG nodes that emits references to single promises
pub struct Iter<'a> {
    stack: Vec<PromiseDAGRef<'a>>,
}

impl<'a> Iter<'a> {
    fn new(d: &'a PromiseDAG) -> Self {
        Self {
            stack: vec![PromiseDAGRef::from(d)],
        }
    }
}

impl<'a> IntoIterator for &'a PromiseDAG {
    type Item = &'a PromiseSingle;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a PromiseSingle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let d = self.stack.last_mut()?;

            if let Some((last, rest)) = d.then.split_last() {
                d.then = rest;
                return Some(last);
            }

            if let Some((last, rest)) = d.after.split_last() {
                d.after = rest;
                self.stack.push(PromiseDAGRef::from(last));
                continue;
            }

            let d = self.stack.pop();
            debug_assert!(d.as_ref().is_some_and(PromiseDAGRef::is_empty));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.stack.last().map_or(0, |d| d.then.len()), None)
    }
}

struct PromiseDAGRef<'a> {
    after: &'a [PromiseDAG],
    then: &'a [PromiseSingle],
}

impl PromiseDAGRef<'_> {
    const fn is_empty(&self) -> bool {
        self.after.is_empty() && self.then.is_empty()
    }
}

impl<'a> From<&'a PromiseDAG> for PromiseDAGRef<'a> {
    fn from(d: &'a PromiseDAG) -> Self {
        Self {
            after: &d.after,
            then: &d.then,
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{borsh, env};
    use rstest::rstest;

    use super::{super::tests::p, *};

    #[rstest]
    #[case(PromiseDAG::default(), [])]
    #[case(p(1), [p(1)])]
    #[case(
        p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)),
        [p(1), p(2), p(3), p(4), p(5), p(6)],
    )]
    fn test_into_iter(
        #[case] d: impl Into<PromiseDAG>,
        #[case] expected: impl Into<Vec<PromiseSingle>>,
    ) {
        let mut ps = d.into().into_iter().collect::<Vec<_>>();
        let mut expected = expected.into();

        // sort by hashes
        ps.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));
        expected.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));

        assert_eq!(ps, expected);
    }

    #[rstest]
    #[case(PromiseDAG::default(), [])]
    #[case(p(1), [p(1)])]
    #[case(
        p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)),
        [p(1), p(2), p(3), p(4), p(5), p(6)],
    )]
    fn test_iter(
        #[case] d: impl Into<PromiseDAG>,
        #[case] expected: impl Into<Vec<PromiseSingle>>,
    ) {
        let d = d.into();
        let expected = expected.into();

        let mut ps = d.iter().collect::<Vec<_>>();
        let mut expected = expected.iter().collect::<Vec<_>>();

        // sort by hashes
        ps.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));
        expected.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));

        assert_eq!(ps, expected);
    }
}
