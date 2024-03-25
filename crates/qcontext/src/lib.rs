extern crate self as qcontext;
use std::{
  borrow::Borrow,
  cell::UnsafeCell,
  mem::{ManuallyDrop, MaybeUninit},
  ops::{Deref, DerefMut},
};

use qcell::{TCell, TCellOwner};
pub use qcontext_derive::Context;

/// Container for [`Context::State`]
pub struct OnceCell<T> {
  inner: UnsafeCell<MaybeUninit<T>>,
}

impl<T> OnceCell<T> {
  pub const fn new() -> Self {
    OnceCell {
      inner: UnsafeCell::new(MaybeUninit::uninit()),
    }
  }
}

unsafe impl<T> Send for OnceCell<T> where T: Send {}
unsafe impl<T> Sync for OnceCell<T> where T: Sync {}

/// Borrow-owner of all [`TCell<T>`]
pub struct ContextOwner<T: Context>(ManuallyDrop<TCellOwner<T>>);

impl<T> Deref for ContextOwner<T>
where
  T: Context,
{
  type Target = TCellOwner<T>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for ContextOwner<T>
where
  T: Context,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

/// Context for singleton state
pub trait Context: Sized + 'static {
  type State: 'static;

  /// Initialize Context state and acquire a singleton [`ContextOwner`] as the borrow-owner. This
  /// can only ever be called once per process and will panic otherwise
  fn init(state: Self::State) -> ContextOwner<Self> {
    let owner = ContextOwner(ManuallyDrop::new(TCellOwner::new()));

    unsafe { (&mut *Self::context().inner.get()).write(state) };

    owner
  }

  /// Static state whose contents is owned, for borrowing purposes, by the [`ContextOwner`] created
  /// with [`Context::init`]
  fn context() -> &'static OnceCell<Self::State>;

  #[allow(unused_variables)]
  fn state(owner: &ContextOwner<Self>) -> &'static Self::State {
    unsafe { (&*Self::context().inner.get()).assume_init_ref() }
  }
}

/// Extension trait for borrowing from [`Context`]
pub trait ContextExt<T>: Context {
  /// Immutably borrow the contents of a [`TCell`] owned by [`Context`]
  fn get<'a>(owner: &'a ContextOwner<Self>) -> &'a T;

  /// Mutably borrow the contents of a [`TCell`] owned by [`Context`]
  fn get_mut<'a>(owner: &'a mut ContextOwner<Self>) -> &'a mut T;
}

impl<T, C> ContextExt<T> for C
where
  C: Context,
  C::State: Borrow<TCell<C, T>>,
{
  fn get<'a>(owner: &'a ContextOwner<C>) -> &'a T {
    let state = unsafe { (&*C::context().inner.get()).assume_init_ref() };
    let cell: &'a TCell<C, T> = state.borrow();
    cell.ro(&owner)
  }

  fn get_mut<'a>(owner: &'a mut ContextOwner<C>) -> &'a mut T {
    let state = unsafe { (&*C::context().inner.get()).assume_init_ref() };
    let cell: &'a TCell<C, T> = state.borrow();
    cell.rw(owner)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_initializes_and_mutates() {
    #[derive(Context)]
    #[context(state = "TCell<Counter, usize>")]
    struct Counter;

    let mut owner = Counter::init(TCell::new(0));

    *Counter::get_mut(&mut owner) = 9999;

    assert_eq!(Counter::get(&owner), &9999);
  }
}
