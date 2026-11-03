// Ports test/util/lru_cache.test.cpp.

use mln_util::lru_cache::LRU;

// gtest: LRU.LRU
#[test]
fn lru() {
    let mut lru: LRU<i32> = LRU::new();
    assert!(lru.empty());
    lru.touch(1);
    lru.touch(2);
    lru.touch(3);
    assert!(!lru.empty());
    assert_eq!(lru.size(), 3);
    lru.remove(&2);
    lru.remove(&77);
    assert_eq!(lru.evict(), Ok(1));
    assert!(!lru.empty());
    assert_eq!(lru.size(), 1);
    lru.touch(4);
    assert_eq!(lru.evict(), Ok(3));
    assert!(!lru.empty());
    assert_eq!(lru.size(), 1);
    assert!(!lru.is_hit(&3));
    assert!(!lru.is_hit(&33));
    assert!(lru.is_hit(&4));
    assert_eq!(lru.evict(), Ok(4));
    assert!(lru.empty());
    assert!(!lru.is_hit(&4));
    // EXPECT_THROW(lru.evict(), std::runtime_error)
    assert!(lru.evict().is_err());
}
