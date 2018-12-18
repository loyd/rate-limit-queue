use super::*;

#[test]
fn it_should_spread_big_chunk() {
    let quantum = 10;
    let interval = Duration::from_millis(100);
    let coef = 10;

    let mut queue = RateLimitQueue::new(quantum, interval);

    for i in 0..coef * quantum {
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
    let quantum = 10;
    let interval = Duration::from_millis(100);

    let mut queue = RateLimitQueue::new(quantum, interval);

    queue.extend(0..2 * quantum);

    for i in 0..quantum {
        assert_eq!(queue.try_dequeue(), DequeueResult::Data(i));
    }

    assert!(queue.try_dequeue().is_limit());

    thread::sleep(3 * interval);

    for i in quantum..2 * quantum {
        assert_eq!(queue.try_dequeue(), DequeueResult::Data(i));
    }

    assert_eq!(queue.try_dequeue(), DequeueResult::Empty);
}

#[test]
fn it_should_change_allowance_during_iter() {
    let quantum = 2;
    let interval = Duration::from_millis(100);

    let mut queue = RateLimitQueue::new(quantum, interval);
    queue.enqueue(0);
    queue.enqueue(1);
    queue.enqueue(2);

    let it = queue.iter();
    drop(it);

    let actual: Vec<&u32> = queue.iter().collect();
    assert_eq!(&actual, &[&0, &1]);

    assert!(queue.try_dequeue().is_limit());
}
