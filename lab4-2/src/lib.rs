use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    vec,
};

/// `InputCellId` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(usize);
/// `ComputeCellId` is a unique identifier for a compute cell.
/// Values of type `InputCellId` and `ComputeCellId` should not be mutually assignable,
/// demonstrated by the following tests:
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input: react::ComputeCellId = r.create_input(111);
/// ```
///
/// ```compile_fail
/// let mut r = react::Reactor::new();
/// let input = r.create_input(111);
/// let compute: react::InputCellId = r.create_compute(&[react::CellId::Input(input)], |_| 222).unwrap();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellId {
    Input(InputCellId),
    Compute(ComputeCellId),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}

struct Computer<'a, T> {
    subscribers: Vec<CellId>,
    dependencies: Vec<CellId>,
    callbacks: HashSet<CallbackId>,
    compute: Option<Box<dyn Fn(&[T]) -> T + 'a>>,
    notify_resolved: bool,
    value: T,
}

pub struct Reactor<'a, T> {
    // Just so that the compiler doesn't complain about an unused type parameter.
    // You probably want to delete this field.
    cell_map: HashMap<CellId, Computer<'a, T>>,
    callback_map: HashMap<CallbackId, Box<dyn FnMut(T) + 'a>>,
    next_id: usize,
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'a, T> Reactor<'a, T>
where
    T: Copy + PartialEq,
{
    pub fn new() -> Self {
        Self {
            cell_map: HashMap::new(),
            callback_map: HashMap::new(),
            next_id: 0,
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        return id;
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellId {
        let input = InputCellId(self.next_id());
        let cell = CellId::Input(input);

        let computer = Computer {
            subscribers: vec![],
            dependencies: vec![],
            callbacks: HashSet::new(),
            notify_resolved: true,
            compute: None,
            value: initial,
        };

        self.cell_map.insert(cell, computer);
        return input;
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F: Fn(&[T]) -> T + 'a>(
        &mut self,
        dependencies: &[CellId],
        compute_func: F,
    ) -> Result<ComputeCellId, CellId> {
        for dep in dependencies {
            if !self.cell_map.contains_key(dep) {
                return Err(*dep);
            }
        }

        let compute = ComputeCellId(self.next_id());
        let cell = CellId::Compute(compute);

        let mut values = vec![];

        /* subscribe cell to it's dependencies */
        for dep in dependencies {
            let dep_computer = self.cell_map.get_mut(dep).unwrap();
            dep_computer.subscribers.push(cell);
            values.push(dep_computer.value);
        }

        let value = compute_func(&values);

        let computer = Computer {
            subscribers: vec![],
            dependencies: dependencies.to_owned(),
            callbacks: HashSet::new(),
            notify_resolved: true,
            compute: Some(Box::new(compute_func)),
            value,
        };

        self.cell_map.insert(cell, computer);

        return Ok(compute);
    }

    fn mark(&mut self, subscribers: &Vec<CellId>) {
        for sub in subscribers {
            let comp = self.cell_map.get_mut(sub).unwrap();
            comp.notify_resolved = false;

            let sub = comp.subscribers.clone();
            self.mark(&sub);
        }
    }

    fn notify(&mut self, id: CellId) {
        let computer = self.cell_map.get(&id).unwrap();
        println!(
            "id: {:?}, dep: {:#?}, sub: {:#?}",
            id, computer.dependencies, computer.subscribers
        );

        let mut values = vec![];
        for dep in &computer.dependencies {
            let comp = self.cell_map.get(dep).unwrap();
            /* If any depency is in unresolved state quit */
            if comp.notify_resolved == false {
                return;
            }
            let value = comp.value;
            values.push(value);
        }

        let mut execute_callbacks = false;
        let computer = self.cell_map.get_mut(&id).unwrap();
        let value = computer.compute.as_ref().and_then(|f| Some(f(&values)));

        if let Some(val) = value {
            if computer.value != val {
                execute_callbacks = true;
            }
            computer.value = val;
        }

        computer.notify_resolved = true;

        if execute_callbacks {
            let computer = self.cell_map.get(&id).unwrap();
            let callbacks = computer.callbacks.clone();
            let value = computer.value;
            self.execute_callbacks(value, callbacks.into_iter());

            let sub = self.cell_map.get(&id).unwrap().subscribers.clone();
            sub.into_iter().for_each(|s| self.notify(s));
        }
    }

    fn execute_callbacks(&mut self, value: T, callbacks: impl Iterator<Item = CallbackId>) {
        callbacks.for_each(|c_id| {
            let callback = self.callback_map.get_mut(&c_id).unwrap();
            callback(value);
        })
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellId) -> Option<T> {
        self.cell_map.get(&id).and_then(|c| Some(c.value))
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, id: InputCellId, new_value: T) -> bool {
        let comp = match self.cell_map.get_mut(&CellId::Input(id)) {
            None => return false,
            Some(c) => c,
        };

        comp.value = new_value;

        let sub = comp.subscribers.clone();

        self.mark(&sub);
        sub.iter().for_each(|s| self.notify(*s));

        true
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F: FnMut(T) + 'a>(
        &mut self,
        id: ComputeCellId,
        callback: F,
    ) -> Option<CallbackId> {
        if !self.cell_map.contains_key(&CellId::Compute(id)) {
            return None;
        }

        let cb = CallbackId(self.next_id());
        self.callback_map.insert(cb, Box::new(callback));
        self.cell_map
            .get_mut(&CellId::Compute(id))
            .unwrap()
            .callbacks
            .insert(cb);

        return Some(cb);
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(
        &mut self,
        cell: ComputeCellId,
        callback: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        match self.cell_map.get_mut(&CellId::Compute(cell)) {
            None => Err(RemoveCallbackError::NonexistentCell),
            Some(comp) => {
                if comp.callbacks.remove(&callback) {
                    Ok(())
                } else {
                    Err(RemoveCallbackError::NonexistentCallback)
                }
            }
        }
    }
}
