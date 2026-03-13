use crate::Map;

pub trait IterableMap: Map {
    type Keys<'a>: Iterator<Item = &'a Self::K>
    where
        Self: 'a;

    type Values<'a>: Iterator<Item = &'a Self::V>
    where
        Self: 'a;
    type ValuesMut<'a>: Iterator<Item = &'a mut Self::V>
    where
        Self: 'a;

    type Iter<'a>: Iterator<Item = (&'a Self::K, &'a Self::V)>
    where
        Self: 'a;
    type IterMut<'a>: Iterator<Item = (&'a Self::K, &'a mut Self::V)>
    where
        Self: 'a;

    type Drain<'a>: Iterator<Item = (Self::K, Self::V)>
    where
        Self: 'a;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn keys(&self) -> Self::Keys<'_>;

    fn values(&self) -> Self::Values<'_>;
    fn values_mut(&mut self) -> Self::ValuesMut<'_>;

    fn iter(&self) -> Self::Iter<'_>;
    fn iter_mut(&mut self) -> Self::IterMut<'_>;

    fn drain(&mut self) -> Self::Drain<'_>;
    fn clear(&mut self);
}
