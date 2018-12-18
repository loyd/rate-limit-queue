#![recursion_limit = "128"]

#[macro_use]
extern crate delegate;

use std::{
    collections::VecDeque,
    iter::Extend,
    thread,
    time::{Duration, Instant},
};

#[cfg(test)]
mod tests;

/// A rate limited queue.
pub struct RateLimitQueue<T> {
    quantum: usize,
    interval: Duration,
    queue: VecDeque<T>,
    allowance: usize,
    timepoint: Instant,
}

/// A type that represents result of `try_dequeue()`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum DequeueResult<T> {
    Data(T),
    Empty,
    Limit(Duration),
}

impl<T> DequeueResult<T> {
    pub fn is_data(&self) -> bool {
        match self {
            DequeueResult::Data(_) => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            DequeueResult::Empty => true,
            _ => false,
        }
    }

    pub fn is_limit(&self) -> bool {
        match self {
            DequeueResult::Limit(_) => true,
            _ => false,
        }
    }
}

impl<T> From<Option<T>> for DequeueResult<T> {
    fn from(opt: Option<T>) -> DequeueResult<T> {
        opt.map_or(DequeueResult::Empty, DequeueResult::Data)
    }
}

impl<T> Into<Option<T>> for DequeueResult<T> {
    fn into(self) -> Option<T> {
        match self {
            DequeueResult::Data(value) => Some(value),
            DequeueResult::Empty | DequeueResult::Limit(_) => None,
        }
    }
}

impl<T> RateLimitQueue<T> {
    /// Creates an empty queue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::RateLimitQueue;
    ///
    /// let queue: RateLimitQueue<i32> = RateLimitQueue::new(100, Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn new(quantum: usize, interval: Duration) -> RateLimitQueue<T> {
        RateLimitQueue::with_capacity(0, quantum, interval)
    }

    /// Creates an empty `VecDeque` with space for at least `n` elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::RateLimitQueue;
    ///
    /// let queue: RateLimitQueue<u32> = RateLimitQueue::with_capacity(10, 100, Duration::from_secs(1));
    /// ```
    #[inline]
    pub fn with_capacity(cap: usize, quantum: usize, interval: Duration) -> RateLimitQueue<T> {
        RateLimitQueue {
            quantum,
            interval,
            queue: VecDeque::with_capacity(cap),
            allowance: quantum,
            timepoint: Instant::now(),
        }
    }

    delegate! {
        target self.queue {
            /// Returns the number of elements the queue can hold without reallocating.
            pub fn capacity(&mut self) -> usize;
            /// Reserves the minimum capacity for exactly `additional` more elements.
            ///
            /// # Panics
            ///
            /// Panics if the new capacity overflows `usize`.
            pub fn reserve_exact(&mut self, additional: usize);
            /// Reserves capacity for at least `additional` more elements.
            ///
            /// # Panics
            ///
            /// Panics if the new capacity overflows `usize`.
            pub fn reserve(&mut self, additional: usize);
            /// Shrinks the capacity of the queue as much as possible.
            pub fn shrink_to_fit(&mut self);
            /// Shortens the queue, dropping excess elements from the back.
            pub fn truncate(&mut self, len: usize);
            /// Returns the number of elements in the queue.
            pub fn len(&self) -> usize;
            /// Returns `true` if the queue is empty.
            pub fn is_empty(&self) -> bool;
        }
    }

    /// Changes the quantum.
    pub fn set_quantum(&mut self, quantum: usize) {
        self.quantum = quantum;
    }

    /// Changes the interval.
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Appends an element to the back of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::RateLimitQueue;
    ///
    /// let mut queue = RateLimitQueue::new(100, Duration::from_secs(1));
    /// queue.enqueue(1);
    /// queue.enqueue(2);
    /// ```
    pub fn enqueue(&mut self, value: T) {
        self.queue.push_back(value);
    }

    /// Removes the first element and returns it, or `None` if the queue is empty.
    ///
    /// Sleeps if the limit has been reached.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::RateLimitQueue;
    ///
    /// let mut queue = RateLimitQueue::new(100, Duration::from_secs(1));
    /// queue.enqueue(1);
    /// queue.enqueue(2);
    ///
    /// assert_eq!(queue.dequeue(), Some(1));
    /// assert_eq!(queue.dequeue(), Some(2));
    /// ```
    pub fn dequeue(&mut self) -> Option<T> {
        match self.try_dequeue() {
            DequeueResult::Data(value) => Some(value),
            DequeueResult::Empty => None,
            DequeueResult::Limit(rest) => {
                thread::sleep(rest);

                if let DequeueResult::Data(value) = self.try_dequeue() {
                    Some(value)
                } else {
                    unreachable!()
                }
            }
        }
    }

    /// Tries to remove the first element and return it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::{RateLimitQueue, DequeueResult};
    ///
    /// let mut queue = RateLimitQueue::new(2, Duration::from_secs(10));
    /// queue.enqueue(1);
    /// queue.enqueue(2);
    ///
    /// assert_eq!(queue.try_dequeue(), DequeueResult::Data(1));
    /// assert_eq!(queue.try_dequeue(), DequeueResult::Data(2));
    /// assert_eq!(queue.try_dequeue(), DequeueResult::Empty);
    ///
    /// queue.enqueue(3);
    /// assert!(queue.try_dequeue().is_limit());
    /// ```
    pub fn try_dequeue(&mut self) -> DequeueResult<T> {
        if self.queue.is_empty() {
            return DequeueResult::Empty;
        }

        if self.allowance > 0 {
            self.allowance -= 1;
            return self.queue.pop_front().into();
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.timepoint);

        match self.interval.checked_sub(elapsed) {
            Some(rest) => DequeueResult::Limit(rest),
            None => {
                self.allowance = self.quantum - 1;
                self.timepoint = now;
                self.queue.pop_front().into()
            }
        }
    }

    /// Returns a front-to-back iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::{RateLimitQueue, DequeueResult};
    ///
    /// let mut queue = RateLimitQueue::new(2, Duration::from_secs(10));
    /// queue.enqueue(1);
    /// queue.enqueue(2);
    ///
    /// let b: &[_] = &[&1, &2];
    /// let c: Vec<&i32> = queue.iter().collect();
    /// assert_eq!(&c[..], b);
    /// ```
    pub fn iter(&mut self) -> impl Iterator<Item = &T> {
        self.queue.iter().take(self.allowance)
    }

    /// Returns a front-to-back iterator that returns mutable references.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::time::Duration;
    /// use rate_limit_queue::{RateLimitQueue, DequeueResult};
    ///
    /// let mut queue = RateLimitQueue::new(2, Duration::from_secs(10));
    /// queue.enqueue(1);
    /// queue.enqueue(2);
    ///
    /// let b: &[_] = &[&mut 1, &mut 2];
    /// let c: Vec<&mut i32> = queue.iter_mut().collect();
    /// assert_eq!(&c[..], b);
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.queue.iter_mut().take(self.allowance)
    }
}

impl<T> Extend<T> for RateLimitQueue<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.queue.extend(iter)
    }
}
