use crate::{PromiseDAG, PromiseSingle};

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

    /// Returns an iterator over all single promises in arbitrary order
    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}

impl Iterator for IntoIter {
    type Item = PromiseSingle;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.stack.last_mut()?.promises.pop() {
                return Some(p);
            }
            let d = self.stack.pop()?;
            self.stack.extend(d.after);
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.stack.last().map_or(0, |d| d.promises.len()), None)
    }
}

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
            if let Some((last, rest)) = d.promises.split_last() {
                d.promises = rest;
                return Some(last);
            }

            let d = self.stack.pop()?;
            self.stack.extend(d.after.iter().map(PromiseDAGRef::from));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.stack.last().map_or(0, |d| d.promises.len()), None)
    }
}

struct PromiseDAGRef<'a> {
    after: &'a [PromiseDAG],
    promises: &'a [PromiseSingle],
}

impl<'a> From<&'a PromiseDAG> for PromiseDAGRef<'a> {
    fn from(d: &'a PromiseDAG) -> Self {
        Self {
            after: &d.after,
            promises: &d.promises,
        }
    }
}
