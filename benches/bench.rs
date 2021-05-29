use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pathtrie::PathTrie;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::{BTreeMap, HashMap};

fn rand_string() -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    format!("common_prefix/{}", rand_string)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("Maps");
    g.bench_function("hashmap", |b| {
        let mut m = HashMap::new();
        for i in 1u64..=10000 {
            m.insert(rand_string(), i);
        }
        b.iter(|| m.get(black_box(&rand_string())))
    });

    g.bench_function("btreemap", |b| {
        let mut m = BTreeMap::new();
        for i in 1u64..=10000 {
            m.insert(rand_string(), i);
        }
        b.iter(|| m.get(black_box(&rand_string())))
    });

    g.bench_function("pathtrie", |b| {
        let mut m = PathTrie::new();
        for i in 1u64..=10000 {
            m.insert(rand_string(), i);
        }
        b.iter(|| m.get(black_box(&rand_string())))
    });

    g.bench_function("fst", |b| {
        let mut m = PathTrie::new();
        for i in 1u64..=10000 {
            m.insert(rand_string(), i);
        }
        let fst = m.into_fst().unwrap();

        b.iter(|| fst.get(black_box(&rand_string())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
