use near_sdk::near;

/// A persistent lock, which stores its state (whether it's locked or unlocked)
/// on-chain, so that the inner value can be accessed depending on
/// the current state of the lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct Lock<T> {
    #[serde(
        default,
        // do not serialize `false`
        skip_serializing_if = "::core::ops::Not::not"
    )]
    locked: bool,
    #[serde(flatten)]
    value: T,
}

impl<T> Lock<T> {
    #[must_use]
    #[inline]
    pub const fn new(value: T, locked: bool) -> Self {
        Self { value, locked }
    }

    #[must_use]
    #[inline]
    pub const fn unlocked(value: T) -> Self {
        Self::new(value, false)
    }

    #[must_use]
    #[inline]
    pub const fn locked(value: T) -> Self {
        Self::new(value, true)
    }

    #[inline]
    pub const fn is_locked(&self) -> bool {
        self.locked
    }

    #[inline]
    pub const fn as_locked(&self) -> Option<&T> {
        if !self.locked {
            return None;
        }
        Some(&self.value)
    }

    #[inline]
    pub fn as_locked_mut(&mut self) -> Option<&mut T> {
        if !self.locked {
            return None;
        }
        Some(&mut self.value)
    }

    #[inline]
    pub fn lock(&mut self) -> Option<&mut T> {
        if self.locked {
            return None;
        }
        self.locked = true;
        Some(&mut self.value)
    }

    #[inline]
    pub fn force_lock(&mut self) -> &mut T {
        self.locked = true;
        &mut self.value
    }

    #[inline]
    pub const fn is_unlocked(&self) -> bool {
        !self.locked
    }

    #[inline]
    pub const fn as_unlocked(&self) -> Option<&T> {
        if self.locked {
            return None;
        }
        Some(&self.value)
    }

    #[inline]
    pub fn as_unlocked_mut(&mut self) -> Option<&mut T> {
        if self.locked {
            return None;
        }
        Some(&mut self.value)
    }

    #[inline]
    pub fn unlock(&mut self) -> Option<&mut T> {
        if !self.locked {
            return None;
        }
        self.locked = false;
        Some(&mut self.value)
    }

    #[inline]
    pub fn force_unlock(&mut self) -> &mut T {
        self.locked = false;
        &mut self.value
    }
}

impl<T> From<T> for Lock<T> {
    fn from(value: T) -> Self {
        Self::unlocked(value)
    }
}