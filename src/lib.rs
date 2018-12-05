use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant},
};

pub struct RateLimitQueue<T> {
    rate: usize,
    interval: Duration,
    queue: VecDeque<T>,
    allowance: usize,
    timepoint: Instant,
}

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
        RateLimitQueue {
            rate,
            interval,
            queue: VecDeque::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let rate = 10;
        let interval = Duration::from_millis(100);
        let coef = 10;

        let mut queue = RateLimitQueue::new(rate, interval);

        for i in 0..coef * rate {
            queue.push(i);
        }

        let mut n = 0;
        let start = Instant::now();

        while let Some(item) = queue.wait() {
            assert_eq!(item, n);
            n += 1;
        }

        let spent = start.elapsed();
        let expected = interval * (coef - 1) as u32;

        assert!(spent > expected);
        assert!(spent < expected + interval / 10);
    }
}
