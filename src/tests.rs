use super::*;

#[test]
fn it_should_spread_big_chunk() {
    let rate = 10;
    let interval = Duration::from_millis(100);
    let coef = 10;

    let mut queue = RateLimitQueue::new(rate, interval);

    for i in 0..coef * rate {
        queue.enqueue(i);
    }

    let mut n = 0;
    let start = Instant::now();

    while let Some(item) = queue.dequeue() {
        assert_eq!(item, n);
        n += 1;
    }

    let spent = start.elapsed();
    let expected = interval * (coef - 1) as u32;

    assert!(spent > expected);
    assert!(spent < expected + interval / 10);
}

#[test]
fn it_should_not_have_accumulative_effect() {
    let rate = 10;
    let interval = Duration::from_millis(100);

    let mut queue = RateLimitQueue::new(rate, interval);

    queue.extend(0..2 * rate);

    for i in 0..rate {
        assert_eq!(queue.try_dequeue(), DequeueResult::Data(i));
    }

    match queue.try_dequeue() {
        DequeueResult::Data(_) | DequeueResult::Empty => unreachable!(),
        DequeueResult::Limit(_) => {}
    }

    thread::sleep(3 * interval);

    for i in rate..2 * rate {
        assert_eq!(queue.try_dequeue(), DequeueResult::Data(i));
    }

    match queue.try_dequeue() {
        DequeueResult::Data(_) | DequeueResult::Limit(_) => unreachable!(),
        DequeueResult::Empty => {}
    }
}
