//! Enabling the borrow checker at runtime

use crate::cell::Cell;
use std::ops::DerefMut;
use std::{cell::UnsafeCell, ops::Deref};

#[derive(Clone, Copy)]
enum RefState {
    Unshared,
    Shared(usize),
    Exclusive,
}

/// A smart pointer that ensures the borrow checker at runtime
pub struct RefCell<T> {
    value: UnsafeCell<T>,
    state: Cell<RefState>,
}

impl<T> RefCell<T> {
    /// Create a smart pointer to the given value
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            state: Cell::new(RefState::Unshared),
        }
    }

    /// Borrow the inner value if no exclusive access has been given out
    pub fn borrow(&self) -> Option<Ref<'_, T>> {
        match self.state.get() {
            RefState::Unshared => {
                self.state.set(RefState::Shared(1));
                Some(Ref { refcell: self })
            }
            RefState::Shared(n) => {
                self.state.set(RefState::Shared(n + 1));
                Some(Ref { refcell: self })
            }
            RefState::Exclusive => None,
        }
    }

    /// Take exclusive access to the inner value
    pub fn borrow_mut(&self) -> Option<RefMut<'_, T>> {
        if let RefState::Unshared = self.state.get() {
            self.state.set(RefState::Exclusive);
            Some(RefMut { refcell: self })
        } else {
            None
        }
    }
}

/// A shared reference to a `RefCell`
pub struct Ref<'refcell, T> {
    refcell: &'refcell RefCell<T>,
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        match self.refcell.state.get() {
            RefState::Shared(1) => self.refcell.state.set(RefState::Unshared),
            RefState::Shared(n) => self.refcell.state.set(RefState::Shared(n - 1)),
            RefState::Exclusive | RefState::Unshared => unreachable!(),
        }
    }
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: A `Ref` only exists if no exclusive access to the inner value has been given
        // out, once a `Ref` is created, no exclusive will be given out. Hence, dereferencing into
        // a shared reference is ok.
        unsafe { &*self.refcell.value.get() }
    }
}

/// An exclusive reference to a `RefCell`
pub struct RefMut<'refcell, T> {
    refcell: &'refcell RefCell<T>,
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        match self.refcell.state.get() {
            RefState::Exclusive => self.refcell.state.set(RefState::Unshared),
            RefState::Shared(_) | RefState::Unshared => unreachable!(),
        }
    }
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: See safety for `DerefMut`
        unsafe { &*self.refcell.value.get() }
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        // SAFETY: A `RefMMut` only exists if no reference to the inner value has been given out,
        // once a `RefMut` is created, no future reference will be given out. Hence, dereferencing
        // into a mutable reference is ok.
        unsafe { &mut *self.refcell.value.get() }
    }
}
