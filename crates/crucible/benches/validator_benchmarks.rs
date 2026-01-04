//! Validator performance benchmarks.
//!
//! Measures validation performance for taxonomy, accession, and ontology validators.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use crucible::bio::{AccessionValidator, OntologyValidator, TaxonomyValidator};

/// Sample organism names for taxonomy validation.
const TAXONOMY_SAMPLES: &[&str] = &[
    "Homo sapiens",
    "Escherichia coli",
    "Mus musculus",
    "Saccharomyces cerevisiae",
    "Drosophila melanogaster",
    "E. coli",
    "H. sapiens",
    "homo sapiens",
    "Bacteroides fragilis",
    "Staphylococcus aureus",
    "human",
    "mouse",
    "Caenorhabditis elegans",
    "Arabidopsis thaliana",
    "Danio rerio",
];

/// Sample accessions for validation.
const ACCESSION_SAMPLES: &[&str] = &[
    "SAMN12345678",
    "SRR1234567",
    "PRJNA123456",
    "NM_001234567",
    "NC_000001.11",
    "P53_HUMAN",
    "6LU7",
    "ERR1234567",
    "SAME12345678",
    "XM_001234567",
    "NP_001234567",
    "WP_012345678",
    "invalid_accession",
    "SRR123",
    "SAMN1234",
];

/// Sample terms for ontology mapping.
const ONTOLOGY_SAMPLES: &[&str] = &[
    "blood",
    "soil",
    "ocean",
    "gut",
    "liver",
    "diabetes",
    "cancer",
    "forest",
    "river",
    "human gut metagenome",
    "marine sediment",
    "freshwater",
    "lung tissue",
    "Crohn disease",
    "inflammatory bowel disease",
];

/// Benchmark taxonomy validation.
fn bench_taxonomy_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("taxonomy_validation");
    let validator = TaxonomyValidator::new();

    // Single validation
    group.bench_function("single", |b| {
        b.iter(|| {
            black_box(validator.validate("Escherichia coli"))
        })
    });

    // Batch validation
    group.bench_function("batch_15", |b| {
        b.iter(|| {
            for sample in TAXONOMY_SAMPLES {
                black_box(validator.validate(sample));
            }
        })
    });

    // Abbreviation expansion
    group.bench_function("abbreviation_expansion", |b| {
        b.iter(|| {
            black_box(validator.expand_abbreviation("E. coli"))
        })
    });

    group.finish();
}

/// Benchmark accession validation.
fn bench_accession_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("accession_validation");
    let validator = AccessionValidator::new();

    // Single validation
    group.bench_function("single", |b| {
        b.iter(|| {
            black_box(validator.validate("SAMN12345678"))
        })
    });

    // Batch validation
    group.bench_function("batch_15", |b| {
        b.iter(|| {
            for sample in ACCESSION_SAMPLES {
                black_box(validator.validate(sample));
            }
        })
    });

    // URL generation
    group.bench_function("url_generation", |b| {
        b.iter(|| {
            black_box(validator.get_url("SRR1234567"))
        })
    });

    group.finish();
}

/// Benchmark ontology mapping.
fn bench_ontology_mapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("ontology_mapping");
    let validator = OntologyValidator::new();

    // Lookup by ID
    group.bench_function("lookup_by_id", |b| {
        b.iter(|| {
            black_box(validator.lookup_by_id("ENVO:00001998"))
        })
    });

    // Lookup by label
    group.bench_function("lookup_by_label", |b| {
        b.iter(|| {
            black_box(validator.lookup_by_label("soil"))
        })
    });

    // Suggest mappings (fuzzy matching)
    group.bench_function("suggest_mappings", |b| {
        b.iter(|| {
            black_box(validator.suggest_mappings("human gut", None))
        })
    });

    // Batch suggestions
    group.bench_function("batch_suggestions_15", |b| {
        b.iter(|| {
            for sample in ONTOLOGY_SAMPLES {
                black_box(validator.suggest_mappings(sample, None));
            }
        })
    });

    group.finish();
}

/// Benchmark validator creation overhead.
fn bench_validator_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validator_creation");

    group.bench_function("taxonomy_new", |b| {
        b.iter(|| {
            black_box(TaxonomyValidator::new())
        })
    });

    group.bench_function("accession_new", |b| {
        b.iter(|| {
            black_box(AccessionValidator::new())
        })
    });

    group.bench_function("ontology_new", |b| {
        b.iter(|| {
            black_box(OntologyValidator::new())
        })
    });

    group.finish();
}

/// Benchmark validation with varying input lengths.
fn bench_input_length_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_length_scaling");
    let taxonomy = TaxonomyValidator::new();
    let accession = AccessionValidator::new();

    for len in [10, 50, 100, 500].iter() {
        let input: String = "a".repeat(*len);

        group.bench_with_input(BenchmarkId::new("taxonomy", len), &input, |b, input| {
            b.iter(|| black_box(taxonomy.validate(input)))
        });

        group.bench_with_input(BenchmarkId::new("accession", len), &input, |b, input| {
            b.iter(|| black_box(accession.validate(input)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_taxonomy_validation,
    bench_accession_validation,
    bench_ontology_mapping,
    bench_validator_creation,
    bench_input_length_scaling,
);
criterion_main!(benches);
