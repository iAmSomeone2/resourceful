//! # Budget Module
//!
//! Budget management system for tracking and allocating limited resources.
//!
//! This module provides a flexible budgeting system that allows for safe allocation and tracking
//! of limited resources. The core design revolves around three main components:
//!
//! - **[Budget]**: A trait defining the behavior of budgeting-capable objects
//! - **[Allocation]**: A RAII-style structure that represents an active allocation from a budget
//! - **[BudgetValue]**: A trait for types that can be used as budget values
//!
//! # Core Concepts
//!
//! ## Budgets
//!
//! A [Budget] tracks a limited resource with a maximum capacity. Resources can be allocated from
//! the budget, reducing its available capacity, and are automatically returned when allocations
//! are dropped. This pattern ensures that resources are properly managed without manual tracking.
//!
//! ## Allocations
//!
//! [Allocation]s are RAII guards that represent a portion of a budget that has been allocated.
//! When an allocation is dropped, its amount is automatically returned to the source budget.
//! This automatic cleanup prevents resource leaks and simplifies resource management.
//!
//! ## Budget Values
//!
//! [BudgetValue] defines the requirements for types that can be used to represent budget amounts.
//! All standard unsigned integer types (`u8`, `u16`, `u32`, `u64`, `u128`, `usize`) automatically
//! implement this trait.
//!
//! # Implementations
//!
//! This module provides two built-in [Budget] implementations:
//!
//! - **[SyncBudget]**: A single-threaded implementation using [`Cell`] for interior mutability
//! - **[AtomicBudget]**: A thread-safe implementation using [`AtomicUsize`]
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```
//! use resourceful_core::{Budget, SyncBudget};
//!
//! let budget = SyncBudget::new(100u32);
//!
//! // Allocate some budget
//! let allocation = budget.allocate(30).expect("Failed to allocate");
//! assert_eq!(budget.remaining(), 70);
//!
//! // When the allocation is dropped, the budget is restored
//! drop(allocation);
//! assert_eq!(budget.remaining(), 100);
//! ```
//!
//! ## Preventing Over-allocation
//!
//! ```
//! use resourceful_core::{Budget, SyncBudget};
//!
//! let budget = SyncBudget::new(50u32);
//!
//! let alloc1 = budget.allocate(30).expect("First allocation should succeed");
//! assert_eq!(budget.remaining(), 20);
//!
//! // This allocation fails because there isn't enough remaining capacity
//! let alloc2 = budget.allocate(25);
//! assert!(alloc2.is_none());
//!
//! // After dropping the first allocation, there's enough capacity
//! drop(alloc1);
//! let alloc3 = budget.allocate(25).expect("Allocation should succeed now");
//! ```
//!
//! ## Checking Capacity
//!
//! ```
//! use resourceful_core::{Budget, SyncBudget};
//!
//! let budget = SyncBudget::new(100u32);
//!
//! assert!(budget.has_capacity(50));
//! assert!(budget.is_full());
//! assert!(!budget.is_empty());
//!
//! let _allocation = budget.allocate(100).unwrap();
//! assert!(budget.is_empty());
//! assert!(!budget.is_full());
//! ```
//!
//! # Safety and Correctness
//!
//! The budget system uses saturating arithmetic to prevent overflow and underflow issues.
//! This ensures that budget values always remain within valid ranges, even in edge cases.
//!
//! The RAII pattern for allocations guarantees that resources are properly returned to the
//! budget, even in the presence of panics or early returns.

use num_traits::{ConstZero, SaturatingAdd, SaturatingSub, Unsigned};
use std::cell::Cell;
use std::sync::atomic::AtomicUsize;

/// An unsigned number-like type that can be used to store the underlying value of a [Budget]
///
/// All standard unsigned primitive number types (`u8`, `u16`, `u32`, `u64`, `u128`, `usize`) have
/// automatic implementations of this trait via the blanket impl.
pub trait BudgetValue:
    Unsigned + ConstZero + SaturatingSub + SaturatingAdd + Copy + PartialOrd
{
}

/// Blanket impl for [BudgetValue] trait
#[diagnostic::do_not_recommend]
impl<T> BudgetValue for T where
    T: Unsigned + ConstZero + SaturatingSub + SaturatingAdd + Copy + PartialOrd
{
}

/// Trait defining the expected behavior of a budgeting-capable object
pub trait Budget {
    /// Type which publicly represents the budget's storage.
    ///
    /// # Implementation Notes
    ///
    /// This does not have to be the underlying storage type of a [Budget] implementation. Instead,
    /// it can be a different type that provides a compatible interface for budget operations.
    ///
    /// See [AtomicBudget]'s implementation for an example of this.
    type Value: BudgetValue;

    /// Creates a new [Budget] with the specified maximum allocation.
    fn new(max_allocation: Self::Value) -> Self;

    /// Retrieves the remaining budget value.
    fn remaining(&self) -> Self::Value;

    /// Retrieves the maximum allocation value.
    fn max(&self) -> Self::Value;

    /// Sets the remaining value of the [Budget] to a specific amount.
    fn set(&self, value: Self::Value);

    /// Caps the input value to `self.max()`
    #[inline]
    fn cap_value(&self, value: Self::Value) -> Self::Value {
        if value > self.max() {
            self.max()
        } else {
            value
        }
    }

    /// Returns `true` if the [Budget] has been depleted.
    #[inline]
    fn is_empty(&self) -> bool {
        self.remaining() == Self::Value::ZERO
    }

    /// Returns `true` if the [Budget] is full.
    ///
    /// # Example
    /// ```
    /// use resourceful_core::{Budget, SyncBudget};
    ///
    /// let budget = SyncBudget::new(100u32);
    /// // Budget is full since nothing has been allocated.
    /// assert!(budget.is_full());
    ///
    /// {
    ///     // The budget is no longer full after allocation.
    ///     let allocation = budget.allocate(32).expect("Failed to allocate budget");
    ///     assert!(!budget.is_full());
    /// }
    /// // The budget is full again after all allocations are dropped.
    /// assert!(budget.is_full());
    /// ```
    #[inline]
    fn is_full(&self) -> bool {
        self.remaining() >= self.max()
    }

    /// Returns `true` if the [Budget] has sufficient remaining capacity for the specified amount.
    ///
    /// # Example
    /// ```
    /// use resourceful_core::{SyncBudget, Budget};
    ///
    /// let budget = SyncBudget::new(100u32);
    ///
    /// assert!(budget.has_capacity(50));
    /// assert!(!budget.has_capacity(150));
    /// ```
    #[inline]
    fn has_capacity(&self, amount: Self::Value) -> bool {
        self.remaining() >= amount
    }

    /// Tries to allocate the requested amount from the [Budget].
    ///
    /// # Return Value
    ///
    /// Returns an [Option::Some] containing the [Allocation] if the budget has sufficient capacity,
    /// otherwise returns an [Option::None].
    ///
    /// # Example
    /// ```
    /// use resourceful_core::{SyncBudget, Budget};
    ///
    /// let budget = SyncBudget::new(10u32);
    ///
    /// // Allocating `5` from the budget should succeed since there is capacity
    /// let good_alloc = budget.allocate(5);
    /// assert!(good_alloc.is_some());
    /// let good_alloc = good_alloc.unwrap();
    /// assert_eq!(good_alloc.amount(), 5);
    ///
    /// // This allocation should fail since it will exceed the remaining available capacity
    /// let bad_alloc = budget.allocate(7);
    /// assert!(bad_alloc.is_none());
    ///
    /// // Manually dropping `good_alloc` here so that it's still "in-use" for `bad_alloc`
    /// drop(good_alloc)
    /// ```
    #[must_use = "if the returned allocation is not used, the budget's capacity will not be updated"]
    fn allocate(&'_ self, amount: Self::Value) -> Option<Allocation<'_, Self::Value, Self>>
    where
        Self: Sized,
    {
        if !self.has_capacity(amount) {
            return None;
        }
        let new_value = self.remaining().saturating_sub(&amount);
        self.set(new_value);

        Some(Allocation {
            budget: self,
            amount,
        })
    }

    /// Frees the requested amount back to the budget.
    ///
    /// # Implementor Note
    ///
    /// This function is only intended to be called by the [Allocation] structure's [Drop]
    /// implementation. Using it in any other context will make it difficult to consistently
    /// maintain the [Budget]'s state.
    fn free(&self, allocation: &Allocation<'_, Self::Value, Self>)
    where
        Self: Sized,
    {
        let amount = allocation.amount();
        let new_value = self.remaining().saturating_add(&amount);
        self.set(new_value);
    }
}

/// A structure representing an allocation of a budgeted value.
///
/// This generic structure ties a specific allocation amount to a [Budget] source,
/// ensuring that the allocated value conforms to the constraints of the corresponding [Budget].
pub struct Allocation<'a, T: BudgetValue, B: Budget<Value = T>> {
    budget: &'a B,
    amount: T,
}

impl<T: BudgetValue, B: Budget<Value = T>> Drop for Allocation<'_, T, B> {
    fn drop(&mut self) {
        Budget::free(self.budget, self);
    }
}

impl<T: BudgetValue, B: Budget<Value = T>> Allocation<'_, T, B> {
    /// Returns the allocated amount.
    pub fn amount(&self) -> T {
        self.amount
    }
}

/// A synchronous [Budget] implementation that uses a [BudgetValue] value for storage.
#[derive(Debug)]
pub struct SyncBudget<N: BudgetValue> {
    value: Cell<N>,
    max_alloc: N,
}

impl<N: BudgetValue> Budget for SyncBudget<N> {
    type Value = N;

    fn new(max_allocation: Self::Value) -> Self {
        Self {
            value: Cell::new(max_allocation),
            max_alloc: max_allocation,
        }
    }

    fn remaining(&self) -> Self::Value {
        self.value.get()
    }

    fn max(&self) -> Self::Value {
        self.max_alloc
    }

    fn set(&self, value: Self::Value) {
        let value = self.cap_value(value);
        self.value.set(value);
    }
}

/// An atomic, async-safe [Budget] implementation that uses an [AtomicUsize] value for storage.
///
/// # Async Safety
///
/// Given that this struct uses atomic operations, it is safe to use across multiple threads
/// without external synchronization. Simply wrap it in an `Arc` for thread-safe access.
#[derive(Debug)]
pub struct AtomicBudget {
    value: AtomicUsize,
    max_alloc: usize,
}

impl Budget for AtomicBudget {
    type Value = usize;

    fn new(max_allocation: Self::Value) -> Self {
        AtomicBudget {
            value: AtomicUsize::new(max_allocation),
            max_alloc: max_allocation,
        }
    }

    fn remaining(&self) -> Self::Value {
        self.value.load(std::sync::atomic::Ordering::Acquire)
    }

    fn max(&self) -> Self::Value {
        self.max_alloc
    }

    fn set(&self, value: Self::Value) {
        let value = self.cap_value(value);
        self.value
            .store(value, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    // NOTE: It may be a good idea to have a small 'test_helper' crate if I end up making more of these helper fns.

    fn allocate_test_helper<V: BudgetValue + Debug>(budget: &impl Budget<Value = V>, amount: V) {
        // Check test preconditions
        assert!(
            budget.is_full(),
            "`budget` must be full before running this test"
        );
        let initial_amount = budget.remaining();
        assert!(
            budget.remaining() >= amount,
            "amount ({amount:?}) must be less than or equal to `budget.remaining()` ({initial_amount:?})"
        );

        // Run actual test

        let expected_remainder = budget.remaining() - amount;
        let allocation = budget.allocate(amount);
        assert_eq!(budget.remaining(), expected_remainder);
        drop(allocation);
        assert!(budget.is_full());
    }

    fn overallocation_test_helper<V: BudgetValue + Debug>(
        budget: &impl Budget<Value = V>,
        amount_0: V,
        amount_1: V,
    ) {
        // Check test preconditions
        let initial_amount = budget.remaining();
        assert!(
            initial_amount >= amount_0,
            "amount_0 ({amount_0:?}) must be less than `budget.remaining()` ({initial_amount:?})"
        );
        let expected_remainder = initial_amount - amount_0;
        assert!(
            amount_1 > expected_remainder,
            "amount_1 ({amount_1:?}) must be greater than the remaining budget after `amount_0` is allocated ({expected_remainder:?})"
        );

        // Run actual test

        // Allocating `amount_0` from the budget should succeed since there is capacity
        let good_alloc = budget.allocate(amount_0).expect("First allocation failed");
        assert_eq!(good_alloc.amount(), amount_0);

        // Verify that the previous allocation hasn't fully depleted the budget
        assert!(!budget.is_empty());

        // This allocation should fail since it will exceed the remaining available capacity
        let bad_alloc = budget.allocate(amount_1);
        assert!(bad_alloc.is_none());

        // Manually dropping `good_alloc` here so that it's still "in-use" for `bad_alloc`
        drop(good_alloc);

        /*
         This allocation should succeed for the following reasons:
            - Shadowing the previous `good_alloc` allocation will implicitly drop it, freeing its
              amount from the budget.
            - `budget` should now be full since there are no other active allocations.
        */
        let good_alloc = budget.allocate(amount_1).expect("Second allocation failed");
        assert_eq!(good_alloc.amount(), amount_1);
    }

    #[test]
    fn test_overfill_budget() {
        let budget = SyncBudget::new(100u32);
        budget.set(300);
        assert_eq!(budget.remaining(), 100);
    }

    #[test]
    fn test_sync_budget_allocation() {
        let budget = SyncBudget::new(100u32);
        allocate_test_helper(&budget, 30);
    }

    #[test]
    fn test_sync_budget_overallocation() {
        let budget = SyncBudget::new(10u32);
        overallocation_test_helper(&budget, 5, 7);
    }

    #[test]
    fn test_async_budget_allocation() {
        let budget = AtomicBudget::new(100);
        allocate_test_helper(&budget, 70);
    }

    #[test]
    fn test_async_budget_overallocation() {
        let budget = AtomicBudget::new(10);
        overallocation_test_helper(&budget, 5, 8);
    }
}
