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

pub struct RateLimitQueue<T> {
    rate: usize,
    interval: Duration,
    queue: VecDeque<T>,
    allowance: usize,
    timepoint: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum DequeueResult<T> {
    Data(T),
    Empty,
    Limit(Duration),
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
    pub fn new(rate: usize, interval: Duration) -> RateLimitQueue<T> {
        RateLimitQueue::with_capacity(rate, interval, 0)
    }

    pub fn with_capacity(rate: usize, interval: Duration, cap: usize) -> RateLimitQueue<T> {
        RateLimitQueue {
            rate,
            interval,
            queue: VecDeque::with_capacity(cap),
            allowance: rate,
            timepoint: Instant::now(),
        }
    }

    delegate! {
        target self.queue {
            pub fn capacity(&mut self) -> usize;
            pub fn reserve_exact(&mut self, additional: usize);
            pub fn reserve(&mut self, additional: usize);
            pub fn shrink_to_fit(&mut self);
            pub fn truncate(&mut self, len: usize);
            pub fn len(&self) -> usize;
            pub fn is_empty(&self) -> bool;
        }
    }

    pub fn set_rate(&mut self, rate: usize) {
        self.rate = rate;
    }

    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub fn enqueue(&mut self, value: T) {
        self.queue.push_back(value);
    }

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
                self.allowance = self.rate - 1;
                self.timepoint = now;
                self.queue.pop_front().into()
            }
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &T> {
        self.queue.iter().take(self.allowance)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.queue.iter_mut().take(self.allowance)
    }
}

impl<T> Extend<T> for RateLimitQueue<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.queue.extend(iter)
    }
}
