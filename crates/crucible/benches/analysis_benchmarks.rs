//! Full analysis pipeline performance benchmarks.
//!
//! Measures end-to-end analysis performance including parsing, inference, and validation.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use crucible::{Crucible, MockProvider};
use std::io::Write;
use tempfile::NamedTempFile;

/// Generate realistic biomedical metadata TSV.
fn generate_biomedical_data(rows: usize) -> String {
    let mut data = String::new();

    // Header
    data.push_str("sample_id\torganism\tcollection_date\thost\ttissue\tage\tbmi\tdiagnosis\tbiosample_accession\tsra_accession\n");

    let organisms = ["Homo sapiens", "H. sapiens", "human", "Homo Sapiens"];
    let hosts = ["Homo sapiens", "human", "H. sapiens"];
    let tissues = ["gut", "blood", "liver", "lung", "stool"];
    let diagnoses = ["CD", "UC", "Crohn's disease", "healthy", "IBD"];

    for row in 0..rows {
        // sample_id
        data.push_str(&format!("SAMPLE_{:04}\t", row + 1));
        // organism
        data.push_str(organisms[row % organisms.len()]);
        data.push('\t');
        // collection_date (mixed formats)
        match row % 3 {
            0 => data.push_str(&format!("2023-{:02}-{:02}", (row % 12) + 1, (row % 28) + 1)),
            1 => data.push_str(&format!("{:02}/{:02}/2023", (row % 12) + 1, (row % 28) + 1)),
            _ => data.push_str(&format!("Jan {}, 2023", (row % 28) + 1)),
        }
        data.push('\t');
        // host
        data.push_str(hosts[row % hosts.len()]);
        data.push('\t');
        // tissue
        data.push_str(tissues[row % tissues.len()]);
        data.push('\t');
        // age (with some outliers)
        let age = if row % 50 == 0 { -5 } else if row % 51 == 0 { 200 } else { 25 + (row % 50) as i32 };
        data.push_str(&format!("{}\t", age));
        // bmi
        data.push_str(&format!("{:.1}\t", 18.5 + (row % 20) as f64 * 0.5));
        // diagnosis
        data.push_str(diagnoses[row % diagnoses.len()]);
        data.push('\t');
        // biosample_accession
        data.push_str(&format!("SAMN{:08}\t", 10000000 + row));
        // sra_accession
        data.push_str(&format!("SRR{:07}\n", 1000000 + row));
    }

    data
}

/// Generate minimal data for baseline measurements.
fn generate_minimal_data(rows: usize) -> String {
    let mut data = String::new();
    data.push_str("id\tvalue\n");
    for row in 0..rows {
        data.push_str(&format!("{}\t{}\n", row, row * 2));
    }
    data
}

/// Benchmark full analysis pipeline with biomedical data.
fn bench_full_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_analysis");

    for rows in [10, 50, 100, 500].iter() {
        let data = generate_biomedical_data(*rows);
        let bytes = data.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(BenchmarkId::new("biomedical_rows", rows), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
                    temp.write_all(data.as_bytes()).unwrap();
                    temp
                },
                |temp| {
                    let crucible = Crucible::new().with_llm(MockProvider::new());
                    black_box(crucible.analyze(temp.path()).unwrap())
                },
            )
        });
    }

    group.finish();
}

/// Benchmark analysis with minimal data to measure baseline overhead.
fn bench_analysis_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("analysis_baseline");

    for rows in [10, 100, 1000].iter() {
        let data = generate_minimal_data(*rows);

        group.bench_with_input(BenchmarkId::new("minimal_rows", rows), &data, |b, data| {
            b.iter_with_setup(
                || {
                    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
                    temp.write_all(data.as_bytes()).unwrap();
                    temp
                },
                |temp| {
                    let crucible = Crucible::new().with_llm(MockProvider::new());
                    black_box(crucible.analyze(temp.path()).unwrap())
                },
            )
        });
    }

    group.finish();
}

/// Benchmark Crucible instance creation.
fn bench_crucible_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("crucible_creation");

    group.bench_function("new", |b| {
        b.iter(|| {
            black_box(Crucible::new())
        })
    });

    group.bench_function("with_mock_llm", |b| {
        b.iter(|| {
            black_box(Crucible::new().with_llm(MockProvider::new()))
        })
    });

    group.finish();
}

/// Benchmark analysis with large files (100K, 500K rows).
///
/// These benchmarks test performance at production scale for files up to 100MB.
fn bench_large_file_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_file_analysis");

    // Configure for longer-running benchmarks
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(30));

    // Test with progressively larger files
    // 10K rows ~ 1MB, 100K rows ~ 10MB, 500K rows ~ 50MB
    for rows in [10_000, 100_000].iter() {
        let data = generate_biomedical_data(*rows);
        let bytes = data.len();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("biomedical_rows", rows),
            &data,
            |b, data| {
                b.iter_with_setup(
                    || {
                        let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
                        temp.write_all(data.as_bytes()).unwrap();
                        temp
                    },
                    |temp| {
                        let crucible = Crucible::new().with_llm(MockProvider::new());
                        black_box(crucible.analyze(temp.path()).unwrap())
                    },
                )
            },
        );
    }

    group.finish();
}

/// Benchmark specific components at scale.
///
/// Tests individual analysis components with 100K rows to identify bottlenecks.
fn bench_component_scaling(c: &mut Criterion) {
    use crucible::Parser;

    let mut group = c.benchmark_group("component_scaling");
    group.sample_size(10);

    let rows = 100_000;
    let data = generate_biomedical_data(rows);
    let bytes = data.len();

    // Create temp file once
    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
    temp.write_all(data.as_bytes()).unwrap();
    let path = temp.path().to_path_buf();

    // Benchmark just parsing (no analysis)
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function("parse_100k_rows", |b| {
        b.iter(|| {
            let parser = Parser::new();
            black_box(parser.parse_file(&path).unwrap())
        })
    });

    group.finish();
}

/// Benchmark analysis result processing.
fn bench_result_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_processing");

    // Pre-generate analysis result
    let data = generate_biomedical_data(100);
    let mut temp = NamedTempFile::with_suffix(".tsv").unwrap();
    temp.write_all(data.as_bytes()).unwrap();

    let crucible = Crucible::new().with_llm(MockProvider::new());
    let result = crucible.analyze(temp.path()).unwrap();

    // Benchmark accessing result fields
    group.bench_function("access_schema", |b| {
        b.iter(|| {
            black_box(&result.schema.columns)
        })
    });

    group.bench_function("access_observations", |b| {
        b.iter(|| {
            black_box(&result.observations)
        })
    });

    group.bench_function("count_by_severity", |b| {
        b.iter(|| {
            let errors = result.observations.iter()
                .filter(|o| matches!(o.severity, crucible::Severity::Error))
                .count();
            black_box(errors)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_analysis,
    bench_analysis_baseline,
    bench_crucible_creation,
    bench_result_processing,
);

// Large file benchmarks run separately due to longer execution time
criterion_group!(
    name = large_file_benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(30));
    targets = bench_large_file_analysis, bench_component_scaling
);

criterion_main!(benches, large_file_benches);
