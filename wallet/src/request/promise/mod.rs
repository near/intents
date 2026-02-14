mod action;
mod iter;
mod single;

pub use self::{action::*, iter::*, single::*};

use near_sdk::{Gas, NearToken, Promise, near};

#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromiseDAG {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Self>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub promises: Vec<PromiseSingle>,
}

impl PromiseDAG {
    pub fn new(promise: PromiseSingle) -> Self {
        Self {
            after: Vec::new(),
            promises: vec![promise],
        }
    }

    #[must_use]
    pub fn and(mut self, other: impl Into<Self>) -> Self {
        let other = other.into();
        if self.after.is_empty() && other.after.is_empty() {
            self.promises.extend(other.promises);
            return self;
        }

        Self {
            after: vec![self, other],
            promises: vec![],
        }
    }

    #[must_use]
    pub fn then(self, then: PromiseSingle) -> Self {
        self.then_concurrent([then])
    }

    #[must_use]
    pub fn then_concurrent(mut self, then: impl IntoIterator<Item = PromiseSingle>) -> Self {
        if self.promises.is_empty() {
            self.promises.extend(then);
            return self;
        }

        Self {
            after: vec![self],
            promises: then.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.after.is_empty() && self.promises.is_empty()
    }

    pub fn iter(&self) -> Iter<'_> {
        <&Self as IntoIterator>::into_iter(self)
    }

    /// Returns the length of the longest chain of subsequent action
    /// receipts to be created.
    pub fn depth(&self) -> usize {
        let mut max_depth = 0;
        // store (node, node_depth)
        let mut stack = vec![(self, 0usize)];

        while let Some((d, mut depth)) = stack.pop() {
            depth = depth.saturating_add(d.promises.len().min(1));
            max_depth = max_depth.max(depth);
            stack.extend(d.after.iter().map(|d| (d, depth)));
        }

        max_depth
    }

    /// Returns the total number of action receipts to be created.
    pub fn total_count(&self) -> usize {
        let mut stack = vec![self];
        let mut total: usize = 0;
        while let Some(d) = stack.pop() {
            total = total.saturating_add(d.promises.len());
            stack.extend(&d.after);
        }
        total
    }

    pub fn total_deposit(&self) -> NearToken {
        self.iter()
            .map(PromiseSingle::total_deposit)
            .fold(NearToken::ZERO, NearToken::saturating_add)
    }

    pub fn estimate_gas(&self) -> Gas {
        self.iter()
            .map(PromiseSingle::estimate_gas)
            .fold(Gas::from_gas(0), Gas::saturating_add)
    }

    pub fn normalize(&mut self) {
        // TODO: avoid recursion
        self.after.retain_mut(|after| {
            after.normalize();
            !after.is_empty()
        });
        self.promises.retain(|p| !p.is_empty());
    }

    pub fn build(self) -> Option<Promise> {
        let promises = self.promises.into_iter().filter_map(PromiseSingle::build);

        let Some(after) = self
            .after
            .into_iter()
            // TODO: avoid recursion
            .filter_map(Self::build)
            .reduce(Promise::and)
        else {
            return promises.reduce(Promise::and);
        };

        let mut promises = promises.peekable();
        if promises.peek().is_none() {
            return Some(after);
        }

        // `.then_concurrent([single])` is equivalent to `.then(single)`
        Some(after.then_concurrent(promises).join())
    }
}

impl From<PromiseSingle> for PromiseDAG {
    fn from(promise: PromiseSingle) -> Self {
        Self::new(promise)
    }
}

#[cfg(test)]
mod tests {

    use defuse_test_utils::random::make_arbitrary;
    use near_sdk::{AccountId, borsh, env, serde_json};
    use rstest::rstest;

    use super::*;

    #[test]
    fn and_assosiative() {
        assert_eq!(p(1).and(p(2)).and(p(3)), p(1).and(p(2).and(p(3))));
    }

    #[test]
    fn then_non_assosiative() {
        assert_ne!(p(1).and(p(2)).then(p(3)), p(1).and(p(2).then(p(3))));
    }

    #[rstest]
    #[case(PromiseDAG::default(), 0)]
    #[case(p(1), 1)]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)), 4)]
    fn test_depth(#[case] p: impl Into<PromiseDAG>, #[case] depth: usize) {
        assert_eq!(p.into().depth(), depth);
    }

    #[rstest]
    #[case(PromiseDAG::default(), 0)]
    #[case(p(1), 1)]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)), 6)]
    fn test_total_count(#[case] p: impl Into<PromiseDAG>, #[case] total_count: usize) {
        assert_eq!(p.into().total_count(), total_count);
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
        let mut ps = d.into().into_iter().collect::<Vec<_>>();
        let mut expected = expected.into();

        // sort by hashes
        ps.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));
        expected.sort_by_key(|p| env::sha256(borsh::to_vec(p).unwrap()));

        assert_eq!(ps, expected);
    }

    #[rstest]
    fn test_normalize(#[from(make_arbitrary)] mut d: PromiseDAG) {
        d.normalize();
        check_json(d);
    }

    #[rstest]
    #[case(PromiseDAG::default())]
    #[case(p(1))]
    #[case(p(1).then(p(2)).and(p(3)).then_concurrent([p(4), p(5)]).then(p(6)))]
    fn check_json(#[case] d: impl Into<PromiseDAG>) {
        println!("{}", serde_json::to_string_pretty(&d.into()).unwrap());
    }

    #[rstest]
    fn arbitrary_json(#[from(make_arbitrary)] d: PromiseDAG) {
        check_json(d);
    }

    fn p(n: usize) -> PromiseSingle {
        PromiseSingle::new(format!("p{n}").parse::<AccountId>().unwrap())
    }
}
