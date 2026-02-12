use crate::{PromiseDAG, PromiseDAGRef, PromiseSingle};

impl IntoIterator for PromiseDAG {
    type Item = PromiseSingle;
    type IntoIter = IntoIter;

    /// Returns an iterator over all single promises in arbitrary order
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { stack: vec![self] }
    }
}

#[derive(Debug, Clone)]
pub struct IntoIter {
    stack: Vec<PromiseDAG>,
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

impl<'a> IntoIterator for &'a PromiseDAG {
    type Item = &'a PromiseSingle;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            stack: vec![self.as_ref()],
        }
    }
}

pub struct Iter<'a> {
    stack: Vec<PromiseDAGRef<'a>>,
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
            self.stack.extend(d.after.iter().map(PromiseDAG::as_ref));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.stack.last().map_or(0, |d| d.promises.len()), None)
    }
}
