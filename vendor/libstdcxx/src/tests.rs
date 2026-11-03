// Sanity checks; //difftest/geometry:libstdcxx_difftest compares the exact
// permutations with libstdc++.

use super::*;

fn lcg(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 33
}

#[test]
fn sort_sorts() {
    let mut seed = 1;
    for n in [0usize, 1, 2, 15, 16, 17, 100, 1000] {
        let mut v: Vec<(u64, usize)> = (0..n).map(|i| (lcg(&mut seed) % 10, i)).collect();
        sort_by(&mut v, |a, b| a.0 < b.0);
        assert!(v.windows(2).all(|w| w[0].0 <= w[1].0));
        let mut ids: Vec<usize> = v.iter().map(|p| p.1).collect();
        ids.sort();
        assert_eq!(ids, (0..n).collect::<Vec<_>>());
    }
}

#[test]
fn stable_sort_is_stable() {
    let mut seed = 2;
    for n in [0usize, 1, 2, 7, 8, 14, 15, 100, 1001] {
        let mut v: Vec<(u64, usize)> = (0..n).map(|i| (lcg(&mut seed) % 10, i)).collect();
        let mut expected = v.clone();
        expected.sort_by_key(|p| p.0);
        stable_sort_by(&mut v, |a, b| a.0 < b.0);
        assert_eq!(v, expected);
    }
}

#[test]
fn priority_queue_pops_in_order() {
    let mut seed = 3;
    let mut q = PriorityQueue::new(|a: &u64, b: &u64| a < b);
    let mut values: Vec<u64> = (0..200).map(|_| lcg(&mut seed) % 50).collect();
    for v in &values {
        q.push(*v);
    }
    values.sort_by(|a, b| b.cmp(a));
    let mut popped = Vec::new();
    while !q.is_empty() {
        popped.push(*q.top());
        q.pop();
    }
    assert_eq!(popped, values);
}

#[test]
fn heap_functions() {
    let mut seed = 4;
    let mut v: Vec<u64> = (0..100).map(|_| lcg(&mut seed) % 1000).collect();
    make_heap(&mut v, |a, b| a < b);
    let max = *v.iter().max().unwrap();
    assert_eq!(v[0], max);
    pop_heap(&mut v, |a, b| a < b);
    assert_eq!(*v.last().unwrap(), max);
    v.pop();
    v.push(5000);
    push_heap(&mut v, |a, b| a < b);
    assert_eq!(v[0], 5000);
    sort_heap(&mut v, |a, b| a < b);
    assert!(v.windows(2).all(|w| w[0] <= w[1]));
}
