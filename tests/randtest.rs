use std::collections::BTreeMap;
use rand::Rng;

use sivec::*;

#[test]
fn randtest() {
    const NTESTS: usize = 10000;
    const CAP: usize = 100 * 1024 * 1024;
    let mut backing: BTreeMap<usize, usize> =
        BTreeMap::new();
    let mut testvec: SIVec<usize> =
        SIVec::with_init_fn(CAP, |t| panic!("{} read before write", t));
    let mut prng = rand::thread_rng();
    for _ in 0..NTESTS {
        let i: usize = prng.gen_range(0, CAP);
        let v: usize = prng.gen_range(0, 100000);
        let _ = backing.insert(i, v);
        testvec.set(i, v);
    }
    for (i, v) in backing {
        assert_eq!(v, testvec[i]);
    }
}
