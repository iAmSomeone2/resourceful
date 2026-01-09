use crate::budget::*;

pub trait ResourceValue {
    type Value;

    /// Attempts to acquire a value from this resource.
    fn acquire(&self) -> Option<Self::Value>;

    /// Releases a previously acquired value back to the resource.
    ///
    /// By default, this simply drops the value.
    fn release(&self, value: Self::Value) {
        drop(value);
    }
}

/// Handle to an allocated resource.
pub struct AllocatedResource<'r, T, BV: BudgetValue, B: Budget<Value = BV>> {
    handle: T,
    allocations: Vec<Allocation<'r, BV, B>>,
}

impl<T, BV: BudgetValue, B: Budget<Value = BV>> AsRef<T> for AllocatedResource<'_, T, BV, B> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T, BV: BudgetValue, B: Budget<Value = BV>> AllocatedResource<'_, T, BV, B> {
    /// Gets the underlying resource handle.
    pub fn get(&self) -> &T {
        &self.handle
    }
}

pub trait Resource {
    type Value: ResourceValue;
}

// pub struct Resource<T, BV: BudgetValue, B: Budget<Value = BV>> {
//     value: T,
//     budgets: Vec<B>,
// }
//
// impl<T, BV: BudgetValue, B: Budget<Value = BV>> Resource<T, BV, B> {
//     /// Creates a new unbudgeted resource with the given value.
//     pub fn new(value: T) -> Self {
//         Self {
//             value,
//             budgets: Vec::new(),
//         }
//     }
//
//     /// Adds a budget to this resource, returning the updated result.
//     pub fn with_budget(mut self, budget: B) -> Self {
//         self.budgets.push(budget);
//         self
//     }
//
//     pub fn allocate(&self) -> Option<AllocatedResource<T, BV, B>> {
//         let allocations = self.budgets.map(|budget|)
//     }
// }

// pub trait Resource {
//     /// Type of value this [Resource] represents
//     type Value;
//
//     fn get_one
// }
