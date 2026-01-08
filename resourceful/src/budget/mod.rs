use num_traits::{ConstZero, SaturatingAdd, SaturatingSub, Unsigned};
use std::cell::Cell;

/// An unsigned numeric type that can be used to store the underlying value of a [Budget]
pub trait BudgetValue:
    Unsigned + ConstZero + SaturatingSub + SaturatingAdd + Copy + PartialOrd
{
}

/// Blanket impl for [BudgetValue] trait
impl<T> BudgetValue for T where
    T: Unsigned + ConstZero + SaturatingSub + SaturatingAdd + Copy + PartialOrd
{
}

pub trait Budget: Sized {
    type Value: BudgetValue;

    fn new(max_allocation: Self::Value) -> Self;

    fn remaining(&self) -> Self::Value;

    fn max(&self) -> Self::Value;

    /// Set the remaining value of the budget to a specific amount
    fn set(&self, value: Self::Value);

    fn is_empty(&self) -> bool {
        self.remaining() == Self::Value::ZERO
    }

    fn is_full(&self) -> bool {
        self.remaining() >= self.max()
    }

    fn has_capacity(&self, amount: Self::Value) -> bool {
        self.remaining() >= amount
    }

    fn allocate(&'_ self, amount: Self::Value) -> Option<Allocation<'_, Self::Value, Self>> {
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

    fn free(&self, amount: Self::Value) {
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
        Budget::free(self.budget, self.amount);
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
        let value = if value > self.max_alloc {
            self.max_alloc
        } else {
            value
        };
        self.value.set(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_allocation() {
        let budget = SyncBudget::new(100u32);
        let allocation = budget.allocate(30);
        assert_eq!(budget.remaining(), 70);
        drop(allocation);
        assert!(budget.is_full());
    }
}
