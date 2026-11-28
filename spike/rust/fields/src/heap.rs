//! `std::priority_queue` with libstdc++'s exact heap algorithms.
//!
//! `map::propagate_field` pops equal-distance points in heap order and then
//! picks among them with `random_entry_removed`, so the tie order of the heap
//! is observable. Rust's `BinaryHeap` uses a different sift, so this mirrors
//! `std::__push_heap` / `std::__adjust_heap` from `bits/stl_heap.h`.

pub struct StdPriorityQueue<T, F: Fn(&T, &T) -> bool> {
    c: Vec<T>,
    /// The queue's `Compare`: `comp(a, b)` true means `a` sorts below `b`.
    comp: F,
}

impl<T: Clone, F: Fn(&T, &T) -> bool> StdPriorityQueue<T, F> {
    pub fn new(comp: F) -> Self {
        Self { c: Vec::new(), comp }
    }
    pub fn is_empty(&self) -> bool {
        self.c.is_empty()
    }
    pub fn len(&self) -> usize {
        self.c.len()
    }
    pub fn top(&self) -> &T {
        &self.c[0]
    }
    pub fn push(&mut self, v: T) {
        self.c.push(v);
        let hole = self.c.len() - 1;
        let value = self.c[hole].clone();
        self.push_heap(hole, 0, value);
    }
    pub fn pop(&mut self) {
        let len = self.c.len();
        if len > 1 {
            let last = len - 1;
            let value = self.c[last].clone();
            self.c[last] = self.c[0].clone();
            self.adjust_heap(0, last, value);
        }
        self.c.pop();
    }

    fn push_heap(&mut self, mut hole: usize, top: usize, value: T) {
        while hole > top {
            let parent = (hole - 1) / 2;
            if !(self.comp)(&self.c[parent], &value) {
                break;
            }
            self.c[hole] = self.c[parent].clone();
            hole = parent;
        }
        self.c[hole] = value;
    }

    fn adjust_heap(&mut self, mut hole: usize, len: usize, value: T) {
        let top = hole;
        let mut second = hole;
        while len >= 2 && second < (len - 1) / 2 {
            second = 2 * (second + 1);
            if (self.comp)(&self.c[second], &self.c[second - 1]) {
                second -= 1;
            }
            self.c[hole] = self.c[second].clone();
            hole = second;
        }
        if len & 1 == 0 && len >= 2 && second == (len - 2) / 2 {
            second = 2 * (second + 1);
            self.c[hole] = self.c[second - 1].clone();
            hole = second - 1;
        }
        self.push_heap(hole, top, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_heap_by_first_pops_in_order() {
        let mut q = StdPriorityQueue::new(|a: &(f32, i32), b: &(f32, i32)| a.0 > b.0);
        for (i, d) in [3.0, 1.0, 2.0, 1.0, 0.0, 2.0, 1.0].iter().enumerate() {
            q.push((*d, i as i32));
        }
        let mut out = Vec::new();
        while !q.is_empty() {
            out.push(*q.top());
            q.pop();
        }
        let ds: Vec<f32> = out.iter().map(|p| p.0).collect();
        assert_eq!(ds, vec![0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 3.0]);
    }
}
