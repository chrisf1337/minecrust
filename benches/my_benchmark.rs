use alga::general::SubsetOf;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, ParameterizedBenchmark};
use minecrust::{
    ecs::{entity::Entity, AabbComponent, PrimitiveGeometryComponent, TransformComponent},
    geometry::{Aabb, Ray},
    octree::Node,
    types::prelude::*,
};
use rand::{distributions::Uniform, Rng};
use specs::prelude::*;

fn new_world(n: usize, aabb_size: f32) -> BenchParams {
    let mut world = World::new();
    world.register::<AabbComponent>();
    world.register::<PrimitiveGeometryComponent>();
    world.register::<TransformComponent>();
    let mut rng = rand::thread_rng();
    let uniform_dist = Uniform::new(-aabb_size, aabb_size);

    let mut entities = vec![];
    for _ in 0..n {
        let x = rng.sample(uniform_dist);
        let y = rng.sample(uniform_dist);
        let z = rng.sample(uniform_dist);
        entities.push(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(x, y, z)).to_superset(),
            &world,
        ));
    }

    BenchParams { world, entities }
}

struct BenchParams {
    world: World,
    entities: Vec<Entity>,
}

impl std::fmt::Debug for BenchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} entities", self.entities.len())
    }
}

fn bench_intersect_entity_funcs(c: &mut Criterion) {
    let aabb_size = 50.0;
    let p0 = new_world(1000, aabb_size);
    let p1 = new_world(10000, aabb_size);
    let p2 = new_world(100000, aabb_size);

    let mut rng = rand::thread_rng();
    let uniform_dist = Uniform::new(-aabb_size, aabb_size);
    let direction_dist = Uniform::new(-aabb_size, aabb_size);
    let ray = Ray::new(
        Point3f::new(
            rng.sample(uniform_dist),
            rng.sample(uniform_dist),
            rng.sample(uniform_dist),
        ),
        Vector3f::new(
            rng.sample(direction_dist),
            rng.sample(direction_dist),
            rng.sample(direction_dist),
        ),
    );

    c.bench(
        "Intersect entity",
        ParameterizedBenchmark::new(
            "Naive",
            move |b, BenchParams { world, entities }| {
                b.iter(|| ray.intersect_entities(&entities, &world.read_storage()))
            },
            vec![p0, p1, p2],
        )
        .with_function("Octree", move |b, BenchParams { world, entities }| {
            b.iter_batched(
                || {
                    Node::new_from_entities(
                        &entities,
                        Aabb::new_min_max(
                            Point3f::new(-aabb_size, -aabb_size, -aabb_size),
                            Point3f::new(aabb_size, aabb_size, aabb_size),
                        ),
                        &world.read_storage(),
                    )
                },
                |octree| octree.intersect_entity(&ray, &world.read_storage()),
                BatchSize::SmallInput,
            )
        }),
    );
}

criterion_group!(benches, bench_intersect_entity_funcs);
criterion_main!(benches);
