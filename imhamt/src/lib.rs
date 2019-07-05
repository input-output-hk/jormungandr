#![allow(dead_code)]
#![cfg_attr(feature = "with-bench", feature(test))]
#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

mod bitmap;
mod content;
mod hamt;
mod hash;
mod helper;
mod node;
mod operation;
mod sharedref;

pub use hamt::*;

#[cfg(test)]
mod tests {
    use super::*;

    use quickcheck::{Arbitrary, Gen};

    use std::cmp;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::BTreeMap;
    use std::hash::Hash;

    #[derive(Debug, Clone)]
    enum PlanOperation<K, V> {
        Insert(K, V),
        DeleteOneMatching(usize),
        DeleteOne(usize),
        Update(usize),
        UpdateRemoval(usize),
        Replace(usize, V),
    }

    #[derive(Debug, Clone)]
    struct Plan<K, V>(Vec<PlanOperation<K, V>>);

    const SIZE_LIMIT: usize = 5120;

    impl<K: Arbitrary + Clone + Send, V: Arbitrary + Clone + Send> Arbitrary for Plan<K, V> {
        fn arbitrary<G: Gen>(g: &mut G) -> Plan<K, V> {
            let nb_ops = 100 + cmp::min(SIZE_LIMIT, Arbitrary::arbitrary(g));
            let mut v = Vec::new();
            for _ in 0..nb_ops {
                let op_nb: u32 = Arbitrary::arbitrary(g);
                let op = match op_nb % 6u32 {
                    0 => PlanOperation::Insert(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)),
                    1 => PlanOperation::DeleteOne(Arbitrary::arbitrary(g)),
                    2 => PlanOperation::DeleteOneMatching(Arbitrary::arbitrary(g)),
                    3 => PlanOperation::Update(Arbitrary::arbitrary(g)),
                    4 => PlanOperation::UpdateRemoval(Arbitrary::arbitrary(g)),
                    5 => PlanOperation::Replace(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)),
                    _ => unimplemented!(),
                };
                v.push(op)
            }
            Plan(v)
        }
    }

    #[test]
    fn insert_lookup() {
        let h: Hamt<DefaultHasher, String, u32> = Hamt::new();

        let k1 = "ABC".to_string();
        let v1 = 12u32;

        let k2 = "DEF".to_string();
        let v2 = 24u32;

        let k3 = "XYZ".to_string();
        let v3 = 42u32;

        let h1: Hamt<DefaultHasher, String, u32> = h.insert(k1.clone(), v1).unwrap();
        let h2 = h.insert(k2.clone(), v2).unwrap();

        assert_eq!(h1.lookup(&k1), Some(&v1));
        assert_eq!(h2.lookup(&k2), Some(&v2));
        assert_eq!(h1.lookup(&k2), None);
        assert_eq!(h2.lookup(&k1), None);

        let h3 = h1.insert(k3.clone(), v3).unwrap();

        assert_eq!(h1.lookup(&k3), None);
        assert_eq!(h2.lookup(&k3), None);

        assert_eq!(h3.lookup(&k1), Some(&v1));
        assert_eq!(h3.lookup(&k2), None);
        assert_eq!(h3.lookup(&k3), Some(&v3));

        let (h4, oldk1) = h3.replace(&k1, v3).unwrap();
        assert_eq!(oldk1, v1);
        assert_eq!(h4.lookup(&k1), Some(&v3));
    }

    #[test]
    fn dup_insert() {
        let mut h: Hamt<DefaultHasher, &String, u32> = Hamt::new();
        let dkey = "A".to_string();
        h = h.insert(&dkey, 1).unwrap();
        assert_eq!(
            h.insert(&dkey, 2).and(Ok(())),
            Err(InsertError::EntryExists)
        )
    }

    #[test]
    fn empty_size() {
        let h: Hamt<DefaultHasher, &String, u32> = Hamt::new();
        assert_eq!(h.size(), 0)
    }

    #[test]
    fn delete_key_not_exist() {
        let mut h: Hamt<DefaultHasher, &String, u32> = Hamt::new();
        let dkey = "A".to_string();
        h = h.insert(&dkey, 1).unwrap();
        assert_eq!(
            h.remove_match(&&dkey, &2).and(Ok(())),
            Err(RemoveError::ValueNotMatching)
        )
    }

    #[test]
    fn delete_value_not_match() {
        let mut h: Hamt<DefaultHasher, &String, u32> = Hamt::new();
        let dkey = "A".to_string();
        h = h.insert(&dkey, 1).unwrap();
        assert_eq!(
            h.remove_match(&&dkey, &2).and(Ok(())),
            Err(RemoveError::ValueNotMatching)
        )
    }

    fn next_u32(x: &u32) -> Result<Option<u32>, ()> {
        Ok(Some(x + 1))
    }

    #[test]
    fn delete() {
        let mut h: Hamt<DefaultHasher, String, u32> = Hamt::new();

        let keys = [
            ("KEY1", 10000u32),
            ("KEY2", 20000),
            ("KEY3", 30000),
            ("KEY4", 40000),
            ("KEY5", 50000),
            ("KEY6", 60000),
            ("KEY7", 70000),
            ("KEY8", 80000),
            ("KEY9", 10000),
            ("KEY10", 20000),
            ("KEY11", 30000),
            ("KEY12", 40000),
            ("KEY13", 50000),
            ("KEY14", 60000),
            ("KEY15", 70000),
            ("KEY16", 80000),
        ];

        let k1 = "ABC".to_string();
        let v1 = 12u32;

        let k2 = "DEF".to_string();
        let v2 = 24u32;

        let k3 = "XYZ".to_string();
        let v3 = 42u32;

        for (k, v) in keys.iter() {
            h = h.insert(k.to_string().clone(), *v).unwrap();
        }

        h = h.insert(k1.clone(), v1).unwrap();
        h = h.insert(k2.clone(), v2).unwrap();
        h = h.insert(k3.clone(), v3).unwrap();

        let h2 = h
            .remove_match(&k1, &v1)
            .expect("cannot remove from already inserted");

        assert_eq!(h.size(), 16 + 3);
        assert_eq!(h2.size(), 16 + 2);

        assert_eq!(h.lookup(&k1), Some(&v1));
        assert_eq!(h2.lookup(&k1), None);

        h = h.remove_match(&k2, &v2).unwrap();
        h = h.remove_match(&k3, &v3).unwrap();

        assert_eq!(
            h.remove_match(&k3, &v3).and(Ok(())),
            Err(RemoveError::KeyNotFound),
        );
        assert_eq!(
            h.remove_match(&k1, &v2).and(Ok(())),
            Err(RemoveError::ValueNotMatching),
        );
        assert_eq!(
            h2.insert(k2.clone(), v3).and(Ok(())),
            Err(InsertError::EntryExists)
        );

        assert_eq!(
            h2.update(&"ZZZ".to_string(), next_u32).and(Ok(())),
            Err(UpdateError::KeyNotFound)
        );

        assert_eq!(h.size(), 16 + 1);
        assert_eq!(h2.size(), 16 + 2);
    }

    use hash::HashedKey;
    use std::marker::PhantomData;

    #[test]
    fn collision() {
        let k0 = "keyx".to_string();
        let h1 = HashedKey::compute(PhantomData::<DefaultHasher>, &k0);
        let l = h1.level_index(0);
        let mut found = None;
        for i in 0..10000 {
            let x = format!("key{}", i);
            let h2 = HashedKey::compute(PhantomData::<DefaultHasher>, &"keyx".to_string());
            if h2.level_index(0) == l {
                found = Some(x.clone());
                break;
            }
        }

        match found {
            None => assert!(false),
            Some(x) => {
                let mut h: Hamt<DefaultHasher, String, u32> = Hamt::new();
                h = h.insert(k0.clone(), 1u32).unwrap();
                h = h.insert(x.clone(), 2u32).unwrap();
                assert_eq!(h.size(), 2);
                assert_eq!(h.lookup(&k0), Some(&1u32));
                assert_eq!(h.lookup(&x), Some(&2u32));

                let h2 = h.remove_match(&x, &2u32).unwrap();
                assert_eq!(h2.lookup(&k0), Some(&1u32));
                assert_eq!(h2.size(), 1);

                let h3 = h.remove_match(&k0, &1u32).unwrap();
                assert_eq!(h3.lookup(&x), Some(&2u32));
                assert_eq!(h3.size(), 1);
            }
        }
    }

    fn property_btreemap_eq<A: Eq + Ord + Hash, B: PartialEq>(
        reference: &BTreeMap<A, B>,
        h: &Hamt<DefaultHasher, A, B>,
    ) -> bool {
        // using the btreemap reference as starting point
        for (k, v) in reference.iter() {
            if h.lookup(&k) != Some(v) {
                return false;
            }
        }
        // then asking the hamt for any spurious values
        for (k, v) in h.iter() {
            if reference.get(&k) != Some(v) {
                return false;
            }
        }
        true
    }

    #[quickcheck]
    fn insert_equivalent(xs: Vec<(String, u32)>) -> bool {
        let mut reference = BTreeMap::new();
        let mut h: Hamt<DefaultHasher, String, u32> = Hamt::new();
        for (k, v) in xs.iter() {
            if reference.get(k).is_some() {
                continue;
            }
            reference.insert(k.clone(), v.clone());
            h = h.insert(k.clone(), *v).unwrap();
        }

        property_btreemap_eq(&reference, &h)
    }

    fn get_key_nth<K: Clone, V>(b: &BTreeMap<K, V>, n: usize) -> Option<K> {
        let keys_nb = b.len();
        if keys_nb == 0 {
            return None;
        }
        let mut keys = b.keys();
        Some(keys.nth(n % keys_nb).unwrap().clone())
    }

    #[quickcheck]
    fn plan_equivalent(xs: Plan<String, u32>) -> bool {
        let mut reference = BTreeMap::new();
        let mut h: Hamt<DefaultHasher, String, u32> = Hamt::new();
        //println!("plan {} operations", xs.0.len());
        for op in xs.0.iter() {
            match op {
                PlanOperation::Insert(k, v) => {
                    if reference.get(k).is_some() {
                        continue;
                    }
                    reference.insert(k.clone(), v.clone());
                    h = h.insert(k.clone(), *v).unwrap();
                }
                PlanOperation::DeleteOne(r) => match get_key_nth(&reference, *r) {
                    None => continue,
                    Some(k) => {
                        reference.remove(&k);
                        h = h.remove(&k).unwrap();
                    }
                },
                PlanOperation::DeleteOneMatching(r) => match get_key_nth(&reference, *r) {
                    None => continue,
                    Some(k) => {
                        let v = reference.get(&k).unwrap().clone();
                        reference.remove(&k);
                        h = h.remove_match(&k, &v).unwrap();
                    }
                },
                PlanOperation::Replace(r, newv) => match get_key_nth(&reference, *r) {
                    None => continue,
                    Some(k) => {
                        let v = reference.get_mut(&k).unwrap();
                        *v = newv.clone();

                        h = h.replace(&k, *newv).unwrap().0;
                    }
                },
                PlanOperation::Update(r) => match get_key_nth(&reference, *r) {
                    None => continue,
                    Some(k) => {
                        let v = reference.get_mut(&k).unwrap();
                        *v = *v + 1;

                        h = h.update(&k, next_u32).unwrap();
                    }
                },
                PlanOperation::UpdateRemoval(r) => match get_key_nth(&reference, *r) {
                    None => continue,
                    Some(k) => {
                        reference.remove(&k);
                        h = h.update::<_, ()>(&k, |_| Ok(None)).unwrap();
                    }
                },
            }
        }
        property_btreemap_eq(&reference, &h)
    }

}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::*;

    use std::collections::hash_map::DefaultHasher;
    use std::collections::BTreeMap;

    type Key = String;

    const NB: usize = 1000;

    fn keys() -> Vec<Key> {
        let mut v = Vec::with_capacity(NB);
        for i in 0..NB {
            v.push(format!("key {}", i))
        }
        v
    }

    #[bench]
    fn bench_btreemap_insert(b: &mut test::Bencher) {
        b.iter(|| {
            let mut h: BTreeMap<Key, u32> = BTreeMap::new();
            for k in keys() {
                h.insert(k, 2);
            }
        });
    }

    #[bench]
    fn bench_hamt_insert(b: &mut test::Bencher) {
        b.iter(|| {
            let mut h: Hamt<DefaultHasher, Key, u32> = Hamt::new();
            for k in keys() {
                h = h.insert(k, 2).unwrap()
            }
        });
    }

    #[bench]
    fn bench_btreemap_remove(b: &mut test::Bencher) {
        let mut h: BTreeMap<Key, u32> = BTreeMap::new();
        for k in keys() {
            h.insert(k, 2);
        }
        b.iter(|| {
            let mut h2 = h.clone();
            for k in keys() {
                h2.remove(&k);
            }
        });
    }

    #[bench]
    fn bench_hamt_remove(b: &mut test::Bencher) {
        let mut h: Hamt<DefaultHasher, Key, u32> = Hamt::new();
        for k in keys() {
            h = h.insert(k, 2).unwrap()
        }
        b.iter(|| {
            let mut h2 = h.clone();
            for k in keys() {
                h2 = h2.remove_match(&k, &2).unwrap()
            }
        });
    }
}
