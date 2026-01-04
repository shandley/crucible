# Real-World Validation Test Datasets

These datasets are designed to test Crucible's ability to detect issues that commonly
cause NCBI BioSample/SRA submission rejections. Each dataset contains intentional
errors based on real-world submission problems.

## Dataset Overview

### 1. ncbi_microbiome_submission.tsv

A 16S/metagenome study submission with common gut microbiome errors:

| Row | Issue | NCBI Error Category |
|-----|-------|---------------------|
| MICRO002 | Abbreviated organism "H. sapiens" | Invalid organism |
| MICRO002 | US date format "05/20/2023" | Invalid date |
| MICRO002 | Invalid lat_lon format "37.7749N 122.4194W" | Invalid coordinates |
| MICRO002 | Case inconsistency "Human Gut" vs "human gut" | Inconsistency |
| MICRO003 | Text date format "May 25, 2023" | Invalid date |
| MICRO003 | Missing lat_lon | Missing field |
| MICRO003 | Case error "Homo Sapiens" | Invalid organism |
| MICRO007 | Abbreviated "E. coli" organism | Invalid organism |
| MICRO008 | Non-standard null "NA", "N/A" | Invalid null value |
| MICRO009/010 | Duplicate samples | Duplicate samples |

**Expected detections:** 10+ issues

### 2. ncbi_environmental_submission.tsv

Environmental metagenome samples with MIxS compliance issues:

| Row | Issue | NCBI Error Category |
|-----|-------|---------------------|
| ENV002 | US date format | Invalid date |
| ENV002 | Invalid coordinate format | Invalid coordinates |
| ENV002 | Missing ENVO terms | Missing ontology |
| ENV003 | Missing lat_lon | Missing field |
| ENV006 | "NA" for depth | Invalid null value |
| ENV008 | Abbreviated "B. subtilis" | Invalid organism |
| ENV010 | "n/a", "not applicable" usage | Invalid null value |

**Expected detections:** 7+ issues

### 3. ncbi_human_clinical.tsv

Human clinical study with accession and metadata errors:

| Row | Issue | Category |
|-----|-------|----------|
| CLIN002 | Short SRA accession "SRR123456" (6 digits) | Invalid accession |
| CLIN002 | Abbreviated "H. sapiens" | Invalid organism |
| CLIN002 | US date format | Invalid date |
| CLIN002 | Case inconsistency in host/tissue/disease | Inconsistency |
| CLIN003 | Short BioSample "SAMN1234567" (7 digits) | Invalid accession |
| CLIN004 | Common name "human" for organism | Invalid organism |
| CLIN004 | Abbreviation "T2D" for disease | Inconsistency |
| CLIN006 | Invalid null for date | Invalid null value |
| CLIN006 | Impossible age "-5" | Outlier |
| CLIN007 | Impossible age "200" | Outlier |
| CLIN007 | ERR prefix (EBI) mixed with SAMN (NCBI) | Inconsistency |
| CLIN008/009 | Duplicate samples | Duplicate samples |

**Expected detections:** 12+ issues

## Validation Approach

These datasets are used in `real_world_validation_test.rs` to verify:

1. **Detection rate**: Crucible finds the documented issues
2. **False positive rate**: Crucible doesn't flag correct data
3. **Suggestion quality**: Suggestions match NCBI guidance
4. **Confidence calibration**: High-confidence for clear errors

## NCBI Validation References

- [BioSample Validation Errors](https://www.ncbi.nlm.nih.gov/biosample/docs/submission/validation-errors/)
- [SRA Submission Guidelines](https://www.ncbi.nlm.nih.gov/sra/docs/submit/)
- [MIxS Standards](https://github.com/GenomicsStandardsConsortium/mixs)

## Adding New Test Cases

When adding new real-world test cases:

1. Document the source of the issue (NCBI rejection, user report, etc.)
2. Add expected issues to this README
3. Update the test counts in the validation test
4. Ensure the issue is reproducible
