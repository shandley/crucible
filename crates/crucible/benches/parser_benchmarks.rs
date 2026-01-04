//! Parser performance benchmarks.
//!
//! Measures parsing performance across different file sizes and formats.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use crucible::Parser;
use std::io::Write;
use tempfile::NamedTempFile;

/// Generate synthetic TSV data with the specified number of rows and columns.
fn generate_tsv_data(rows: usize, cols: usize) -> String {
    let mut data = String::new();

    // Header row
    for i in 0..cols {
        if i > 0 {
            data.push('\t');
        }
        data.push_str(&format!("column_{}", i + 1));
    }
    data.push('\n');

    // Data rows
    for row in 0..rows {
        for col in 0..cols {
            if col > 0 {
                data.push('\t');
            }
            // Mix of data types
            match col % 5 {
                0 => data.push_str(&format!("ID_{:06}", row)),
                1 => data.push_str(&format!("{:.2}", row as f64 * 1.5)),
                2 => data.push_str(&format!("2023-{:02}-{:02}", (row % 12) + 1, (row % 28) + 1)),
                3 => data.push_str(if row % 2 == 0 { "true" } else { "false" }),
                4 => data.push_str(&format!("Category_{}", row % 10)),
                _ => unreachable!(),
            }
        }
        data.push('\n');
    }

    data
}

/// Generate synthetic CSV data.
fn generate_csv_data(rows: usize, cols: usize) -> String {
    generate_tsv_data(rows, cols).replace('\t', ",")
}

/// Benchmark parsing TSV files of various sizes.
fn bench_parse_tsv(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_tsv");

    for rows in [100, 1_000, 10_000].iter() {
        let data = generate_tsv_data(*rows, 10);
        let bytes = data.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("rows", rows), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
                    temp.write_all(data.as_bytes()).unwrap();
                    temp
                },
                |temp| {
                    let parser = Parser::new();
                    black_box(parser.parse_file(temp.path()).unwrap())
                },
            )
        });
    }

    group.finish();
}

/// Benchmark parsing CSV files of various sizes.
fn bench_parse_csv(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_csv");

    for rows in [100, 1_000, 10_000].iter() {
        let data = generate_csv_data(*rows, 10);
        let bytes = data.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("rows", rows), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let mut temp = NamedTempFile::with_suffix(".csv").unwrap();
                    temp.write_all(data.as_bytes()).unwrap();
                    temp
                },
                |temp| {
                    let parser = Parser::new();
                    black_box(parser.parse_file(temp.path()).unwrap())
                },
            )
        });
    }

    group.finish();
}

/// Benchmark parsing with varying column counts.
fn bench_parse_column_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_column_scaling");

    let rows = 1_000;
    for cols in [5, 10, 20, 50].iter() {
        let data = generate_tsv_data(rows, *cols);
        let bytes = data.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("cols", cols), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
                    temp.write_all(data.as_bytes()).unwrap();
                    temp
                },
                |temp| {
                    let parser = Parser::new();
                    black_box(parser.parse_file(temp.path()).unwrap())
                },
            )
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_tsv,
    bench_parse_csv,
    bench_parse_column_scaling,
);
criterion_main!(benches);
