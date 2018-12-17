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
pub enum PopResult<T> {
    Data(T),
    Empty,
    Limit(Duration),
}

impl<T> From<Option<T>> for PopResult<T> {
    fn from(opt: Option<T>) -> PopResult<T> {
        opt.map_or(PopResult::Empty, PopResult::Data)
    }
}

impl<T> Into<Option<T>> for PopResult<T> {
    fn into(self) -> Option<T> {
        match self {
            PopResult::Data(value) => Some(value),
            PopResult::Empty | PopResult::Limit(_) => None,
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

    pub fn set_rate(&mut self, rate: usize) {
        self.rate = rate;
    }

    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    pub fn push(&mut self, value: T) {
        self.queue.push_back(value);
    }

    pub fn wait(&mut self) -> Option<T> {
        match self.try_pop() {
            PopResult::Data(value) => Some(value),
            PopResult::Empty => None,
            PopResult::Limit(rest) => {
                thread::sleep(rest);

                if let PopResult::Data(value) = self.try_pop() {
                    Some(value)
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn try_pop(&mut self) -> PopResult<T> {
        if self.queue.is_empty() {
            return PopResult::Empty;
        }

        if self.allowance > 0 {
            self.allowance -= 1;
            return self.queue.pop_front().into();
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.timepoint);

        match self.interval.checked_sub(elapsed) {
            Some(rest) => PopResult::Limit(rest),
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

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T> Extend<T> for RateLimitQueue<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.queue.extend(iter)
    }
}
