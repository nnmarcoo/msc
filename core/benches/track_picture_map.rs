use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
struct ImageHash([u8; 32]);

struct BTreeVersion(BTreeMap<u8, ImageHash>);

impl BTreeVersion {
    fn get(&self, id: u8) -> Option<ImageHash> {
        self.0.get(&id).copied()
    }
}

struct VecVersion(Vec<(u8, ImageHash)>);

impl VecVersion {
    fn get(&self, id: u8) -> Option<ImageHash> {
        self.0.iter().find(|(k, _)| *k == id).map(|(_, h)| *h)
    }
}

struct ArrayVersion {
    data: [(u8, ImageHash); 20],
    len: u8,
}

impl ArrayVersion {
    fn new(items: &[(u8, ImageHash)]) -> Self {
        let mut data = [(0u8, ImageHash::default()); 20];
        for (i, item) in items.iter().enumerate() {
            data[i] = *item;
        }
        Self {
            data,
            len: items.len() as u8,
        }
    }

    fn get(&self, id: u8) -> Option<ImageHash> {
        self.data[..self.len as usize]
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, h)| *h)
    }
}

fn bench_lookup(c: &mut Criterion) {
    let hash = ImageHash([0u8; 32]);

    // Typical case: 1 image (most tracks just have cover art)
    let btree_1: BTreeVersion = BTreeVersion([(3, hash)].into_iter().collect());
    let vec_1: VecVersion = VecVersion(vec![(3, hash)]);
    let arr_1: ArrayVersion = ArrayVersion::new(&[(3, hash)]);

    // Common case: 2-3 images
    let btree_3: BTreeVersion =
        BTreeVersion([(3, hash), (4, hash), (0, hash)].into_iter().collect());
    let vec_3: VecVersion = VecVersion(vec![(3, hash), (4, hash), (0, hash)]);
    let arr_3: ArrayVersion = ArrayVersion::new(&[(3, hash), (4, hash), (0, hash)]);

    // Worst case: 20 images
    let items_20: Vec<_> = (0u8..20).map(|i| (i, hash)).collect();
    let btree_20: BTreeVersion = BTreeVersion(items_20.iter().copied().collect());
    let vec_20: VecVersion = VecVersion(items_20.clone());
    let arr_20: ArrayVersion = ArrayVersion::new(&items_20);

    let mut group = c.benchmark_group("1_item");
    group.bench_function("btree", |b| b.iter(|| btree_1.get(black_box(3))));
    group.bench_function("vec", |b| b.iter(|| vec_1.get(black_box(3))));
    group.bench_function("array", |b| b.iter(|| arr_1.get(black_box(3))));
    group.finish();

    let mut group = c.benchmark_group("3_items_hit_first");
    group.bench_function("btree", |b| b.iter(|| btree_3.get(black_box(3))));
    group.bench_function("vec", |b| b.iter(|| vec_3.get(black_box(3))));
    group.bench_function("array", |b| b.iter(|| arr_3.get(black_box(3))));
    group.finish();

    let mut group = c.benchmark_group("3_items_hit_last");
    group.bench_function("btree", |b| b.iter(|| btree_3.get(black_box(0))));
    group.bench_function("vec", |b| b.iter(|| vec_3.get(black_box(0))));
    group.bench_function("array", |b| b.iter(|| arr_3.get(black_box(0))));
    group.finish();

    let mut group = c.benchmark_group("20_items_hit_first");
    group.bench_function("btree", |b| b.iter(|| btree_20.get(black_box(0))));
    group.bench_function("vec", |b| b.iter(|| vec_20.get(black_box(0))));
    group.bench_function("array", |b| b.iter(|| arr_20.get(black_box(0))));
    group.finish();

    let mut group = c.benchmark_group("20_items_hit_middle");
    group.bench_function("btree", |b| b.iter(|| btree_20.get(black_box(10))));
    group.bench_function("vec", |b| b.iter(|| vec_20.get(black_box(10))));
    group.bench_function("array", |b| b.iter(|| arr_20.get(black_box(10))));
    group.finish();

    let mut group = c.benchmark_group("20_items_hit_last");
    group.bench_function("btree", |b| b.iter(|| btree_20.get(black_box(19))));
    group.bench_function("vec", |b| b.iter(|| vec_20.get(black_box(19))));
    group.bench_function("array", |b| b.iter(|| arr_20.get(black_box(19))));
    group.finish();

    let mut group = c.benchmark_group("20_items_miss");
    group.bench_function("btree", |b| b.iter(|| btree_20.get(black_box(50))));
    group.bench_function("vec", |b| b.iter(|| vec_20.get(black_box(50))));
    group.bench_function("array", |b| b.iter(|| arr_20.get(black_box(50))));
    group.finish();
}

criterion_group!(benches, bench_lookup);
criterion_main!(benches);
