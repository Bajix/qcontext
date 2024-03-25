extern crate self as qcontext;
use std::{
  borrow::Borrow,
  cell::UnsafeCell,
  mem::{ManuallyDrop, MaybeUninit},
  ops::{Deref, DerefMut},
};

#[doc(no_inline)]
pub use qcell::TCell;
use qcell::TCellOwner;
pub use qcontext_derive::Context;

/// Container exclusively for [`Context::State`] and initialized by [`Context::init`]
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

/// Borrow-owner of all [`TCell<T>`] using [`Context`] as the marker type
pub struct ContextOwner<T: Context>(ManuallyDrop<TCellOwner<T>>);

impl<C> ContextOwner<C>
where
  C: Context,
{
  pub fn state(&self) -> &'static C::State {
    C::state(self)
  }

  pub fn get<'a, T>(&'a self) -> &'a T
  where
    C::State: Borrow<TCell<C, T>>,
  {
    C::get(self)
  }

  pub fn get_mut<'a, T>(&'a mut self) -> &'a mut T
  where
    C::State: Borrow<TCell<C, T>>,
  {
    C::get_mut(self)
  }
}

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

#[derive(Context)]
#[context(state = "()")]
/// A stateless global context that acts as the borrow-owner of all [`TCell<Global, T>`]
pub struct Global;

/// Extension trait for borrowing from [`Context::State`]
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
  fn global_context_can_mutate() {
    let counter = TCell::<Global, usize>::new(0);
    let mut owner = Global::init(());

    *counter.rw(&mut owner) = 9999;
    assert_eq!(counter.ro(&owner), &9999);
  }

  #[test]
  fn it_creates_static_references() {
    #[derive(Context)]
    #[context(state = "usize")]
    struct Counter;

    let owner = Counter::init(9999);
    let counter = Counter::state(&owner);
    drop(owner);
    assert_eq!(counter, &9999);
  }

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
