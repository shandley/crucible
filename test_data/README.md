# Test Data for Crucible

This directory contains intentionally messy datasets for testing Crucible's data quality detection capabilities. Each dataset incorporates common data quality issues found in real biomedical research metadata.

## Datasets

### 1. `ibd_cohort_metadata.tsv` - IBD Cohort Study

A simulated inflammatory bowel disease cohort study with 20 samples.

**Intentional Issues:**

| Issue Type | Examples | Rows |
|------------|----------|------|
| **Inconsistent diagnosis values** | CD, Crohn's, Crohns, cd, ulcerative colitis, UC | 3, 5, 6, 14 |
| **Mixed boolean formats** | yes/no, Yes/No, TRUE/FALSE, 1/0, y/n | Throughout |
| **Multiple NA representations** | NA, N/A, missing, empty, NULL, . | 4, 5, 8, 9, 19, 20 |
| **Date format inconsistencies** | 2024-01-15, 01/17/2024, Jan 20 2024, 2024/01/25 | 3, 6, 11 |
| **Duplicate sample_id** | IBD001 appears twice | 1, 13 |
| **Outliers** | age=242, bmi=-5.2 | 17, 14 |
| **Case inconsistencies** | M/m/male/Male, F/f/Female | Throughout |
| **Typos** | stoool (row 18) | 18 |
| **Missing sex value** | Empty string | 9 |

### 2. `microbiome_study_metadata.tsv` - Microbiome Intervention Study

A simulated microbiome study with 20 samples across treatment arms.

**Intentional Issues:**

| Issue Type | Examples | Rows |
|------------|----------|------|
| **Inconsistent body site** | gut, Gut, stool, feces, intestine, GI tract | Throughout |
| **Inconsistent collection method** | swab, Swab, SWAB, fecal_collection, Fecal collection, biopsy | Throughout |
| **Temperature format inconsistencies** | -80C, -80, -80 C, frozen, RT | Throughout |
| **Mixed treatment names** | drug_a, Drug A, Drug_A, drug b, Drug B | Throughout |
| **Multiple NA representations** | NA, empty, not measured, missing, pending | 1, 9, 12, 17, 19 |
| **Outliers** | DNAConc=1250.0, ReadCount=-5000 | 16, 11 |
| **Duplicate PatientID** | P001 appears twice | 1, 15 |
| **Inconsistent response values** | responder, Responder, non-responder, Non-Responder, partial | Throughout |
| **Typos** | non-respnder | 18 |
| **Missing Response values** | Empty in several rows | 1, 7, 17 |

### 3. `clinical_trial_metadata.csv` - Clinical Trial Phase 2

A simulated clinical trial with 20 patients across 3 visits.

**Intentional Issues:**

| Issue Type | Examples | Rows |
|------------|----------|------|
| **Date format inconsistencies** | 2023-06-15, 06/20/2023, July 12 2023 | 3, 11 |
| **Gender inconsistencies** | Male, Female, M, F, male, Unknown | Throughout |
| **Ethnicity inconsistencies** | Caucasian, White, caucasian; African American, African-American, Black, black | Throughout |
| **Mixed boolean formats** | no/yes, No/Yes, FALSE/TRUE, 0/1 | Throughout |
| **Prior treatment inconsistencies** | chemotherapy, Chemotherapy, chemo, chemo + radiation | Throughout |
| **Duplicate patient_id** | PT001 appears twice (visit 1 and 2) | 1, 13 |
| **Outliers** | weight_kg=350.0, age_years=-35 | 16, 19 |
| **Missing/NA values** | NOT RECORDED, empty, NA, n/a, na, unknown | 9, 10, 17, 20 |
| **AE severity inconsistencies** | mild, Mild, moderate, Moderate, severe | Throughout |
| **Boolean as string** | TRUE (row 20, discontinued) | 20 |

## Expected Crucible Detections

Crucible should detect:

1. **Type mismatches** - String values in numeric columns (e.g., "NOT RECORDED" in weight)
2. **Missing patterns** - Various NA representations that should be standardized
3. **Inconsistent categoricals** - Same concept with different representations
4. **Duplicate identifiers** - Non-unique values in ID columns
5. **Outliers** - Values outside reasonable ranges (negative ages, impossible weights)
6. **Format inconsistencies** - Date formats, boolean representations
7. **Typos** - Near-matches to expected values (stoool, non-respnder)

## Usage

```bash
# Analyze with Crucible
cargo run --example analyze -- test_data/ibd_cohort_metadata.tsv
cargo run --example analyze -- test_data/microbiome_study_metadata.tsv
cargo run --example analyze -- test_data/clinical_trial_metadata.csv
```

## Sources

These datasets were designed based on common data quality issues documented in:

- [Five Common Problems with Messy Data](https://www.michaelchimenti.com/2014/07/five-common-problems-with-messy-data/)
- [Metadata harmonization in microbiome research](https://environmentalmicrobiome.biomedcentral.com/articles/10.1186/s40793-022-00425-1)
- [Current challenges in microbiome metadata collection](https://www.biorxiv.org/content/10.1101/2021.05.05.442781v1.full)
- [Variable quality of metadata in biomedical experiments](https://www.nature.com/articles/sdata201921)
