use criterion::black_box;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use jormungandr::rest::v0::logic::get_message_logs;
use jormungandr::rest::Context;
use tokio::runtime::Runtime;

fn tokio() -> Runtime {
    Runtime::new().unwrap()
}

fn empty_context_get_message_logs(c: &mut Criterion) {
    let context = Context::new();

    c.bench_function("empty_context", |b| {
        let f = || get_message_logs(black_box(&context));
        b.to_async(tokio()).iter(f);
    });
}

criterion_group!(benches, empty_context_get_message_logs);
criterion_main!(benches);
