# Crucible Bioinformatics Roadmap

## Overview

This document outlines the plan to add bioinformatics-specific features to Crucible, making it a compelling tool for the research community and a strong candidate for a *Bioinformatics* Application Note publication.

## The Problem

Researchers face significant friction when submitting sequence data to NCBI/ENA:

1. **MIxS Compliance**: The [Minimum Information about any (x) Sequence](https://pmc.ncbi.nlm.nih.gov/articles/PMC3367316/) standard defines mandatory metadata fields, but compliance checking happens at submission time
2. **BioSample Rejections**: [Common validation errors](https://www.ncbi.nlm.nih.gov/biosample/docs/submission/validation-errors/) cause delays and frustration
3. **Ontology Terms**: Free-text entries need mapping to standard ontologies (NCBI Taxonomy, UBERON, ENVO)
4. **No Pre-validation**: Most researchers use spreadsheets without validation until NCBI rejects their submission

The [National Microbiome Data Collaborative](https://pmc.ncbi.nlm.nih.gov/articles/PMC8573954/) explicitly recommends "validating sample metadata with immediate, informative feedback" - Crucible can provide this.

## Solution: Crucible Bio Module

```
┌─────────────────────────────────────────────────────────────────┐
│                    Crucible + Bio Module                        │
├─────────────────────────────────────────────────────────────────┤
│  Validators                                                     │
│  ├── MixsComplianceValidator (MIxS core + environmental pkgs)  │
│  ├── TaxonomyValidator (NCBI Taxonomy validation)              │
│  ├── OntologyTermValidator (UBERON, ENVO, MONDO)               │
│  ├── AccessionValidator (BioSample, SRA, GenBank formats)      │
│  └── BioSamplePreValidator (NCBI submission requirements)       │
├─────────────────────────────────────────────────────────────────┤
│  Suggestion Actions                                             │
│  ├── MapToOntology { term, ontology_id, label }                │
│  ├── AddMissingField { field_name, requirement_level }         │
│  └── StandardizeTaxonomy { input, taxid, scientific_name }     │
├─────────────────────────────────────────────────────────────────┤
│  Domain Context                                                 │
│  ├── microbiome: MIxS-MIMS, ENVO terms, 16S/metagenomics       │
│  ├── genomics: MIxS-MIGS, assembly versions, gene symbols       │
│  └── clinical: human subjects, MONDO disease terms              │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase Bio-1: MIxS Compliance ✅ COMPLETE

**Goal**: Validate metadata against MIxS standard requirements.

#### Deliverables

- [x] MIxS schema loader
  - [x] Define MIxS field structure with requirements (M/C/X/-)
  - [x] Support 15 environmental packages
  - [x] Load core checklist + environmental packages
- [x] MixsComplianceValidator
  - [x] Check mandatory fields (M) are present
  - [x] Validate field formats (date, lat_lon, etc.)
  - [x] Detect appropriate environmental package from context
- [x] Taxonomy validation (bonus)
  - [x] TaxonomyValidator with NCBI taxids
  - [x] Abbreviation expansion (E. coli → Escherichia coli)
  - [x] Fuzzy matching for typos (Levenshtein distance)
- [x] CLI integration
  - [x] `--mixs-package` flag with 15+ package options
  - [x] Verbose mode shows compliance score
  - [x] Bio observations merged into analysis

#### MIxS Core Mandatory Fields

```
investigation_type    # e.g., "metagenome", "genome"
project_name          # Study/project name
lat_lon               # Geographic coordinates
geo_loc_name          # Country and/or sea, region
collection_date       # When sample was collected
env_broad_scale       # Biome (ENVO term)
env_local_scale       # Environmental feature (ENVO term)
env_medium            # Environmental material (ENVO term)
```

#### Environmental Packages

| Package | Use Case |
|---------|----------|
| air | Atmospheric samples |
| built-environment | Indoor/constructed environments |
| host-associated | Samples from host organisms |
| human-associated | Human microbiome samples |
| human-gut | Specifically gut microbiome |
| human-skin | Skin microbiome |
| microbial-mat-biofilm | Microbial mats |
| plant-associated | Plant microbiome |
| sediment | Sediment samples |
| soil | Soil samples |
| wastewater-sludge | Wastewater treatment |
| water | Aquatic samples |

#### Exit Criteria

```bash
$ crucible analyze metadata.tsv --domain microbiome

MIxS Compliance: 60% (4/10 mandatory fields present)

Missing mandatory fields:
  ✗ lat_lon (MIxS core)
  ✗ collection_date (MIxS core)
  ✗ env_broad_scale (MIxS core)
  ✗ env_local_scale (MIxS core)
  ✗ env_medium (MIxS core)
  ✗ samp_collect_device (human-gut package)
```

---

### Phase Bio-2: Taxonomy Validation ✅ COMPLETE

**Goal**: Validate and standardize organism/taxonomy fields.

#### Deliverables

- [x] TaxonomyValidator (expanded)
  - [x] Validate ~150 common organisms (expanded from 20)
  - [x] Model organisms, gut/oral/skin microbiome, pathogens, viruses
  - [x] Major phyla, classes, families, genera
  - [x] Metagenome terms (gut, oral, skin, marine, soil, etc.)
- [x] NCBI Taxonomy data loader
  - [x] `from_ncbi_dump()` method to load names.dmp and nodes.dmp
  - [x] Parse scientific names, common names, ranks, parent taxids
  - [x] Support offline validation with full NCBI database
  - [x] TaxonomyStats for database info
- [x] Taxonomy column auto-detection
  - [x] 25+ column name patterns recognized
  - [x] Exact, partial, and suffix matching
- [x] Observation types (via TaxonomyValidationResult)
  - [x] `Valid` - name matches NCBI
  - [x] `Abbreviation` - should be expanded
  - [x] `CaseError` - incorrect capitalization
  - [x] `PossibleTypo` - fuzzy match found
  - [x] `Unknown` - not in database

#### Common Issues to Detect

| Input | Problem | Suggestion |
|-------|---------|------------|
| `E. coli` | Abbreviation | `Escherichia coli` (taxid:562) |
| `e coli` | Lowercase + no period | `Escherichia coli` (taxid:562) |
| `Homo Sapiens` | Wrong capitalization | `Homo sapiens` (taxid:9606) |
| `Staphylococcus aureus MRSA` | Strain in name | Separate into organism + strain |
| `human` | Common name | `Homo sapiens` (taxid:9606) |
| `mouse` | Common name | `Mus musculus` (taxid:10090) |

#### Exit Criteria

```bash
$ crucible analyze metadata.tsv --domain microbiome

Taxonomy issues found:
  ⚠ Row 3: "E. coli" → suggest "Escherichia coli" (taxid:562)
  ⚠ Row 7: "Bacteroides fragalis" → suggest "Bacteroides fragilis" (taxid:817)
  ✗ Row 12: "Unknown bacterium" → invalid taxonomy
```

---

### Phase Bio-3: Ontology Term Mapping ✅ COMPLETE

**Goal**: Map free-text terms to standard biological ontologies.

#### Deliverables

- [x] Ontology loader (OBO format)
  - [x] ENVO (Environmental Ontology) - ~40 common terms
  - [x] UBERON (Anatomy) - ~50 common terms
  - [x] MONDO (Disease) - ~55 common terms
  - [x] Support for loading additional terms from OBO files
- [x] OntologyValidator
  - [x] Validate existing ontology IDs (format and lookup)
  - [x] Suggest ontology terms for free-text (exact, synonym, partial matching)
  - [x] Column type detection (ENVO, UBERON, MONDO)
- [x] Integration with MIxS validator
  - [x] Automatic detection of ontology-relevant columns
  - [x] Free-text to ontology mapping suggestions
  - [x] Invalid ontology ID detection

#### Priority Ontologies

| Ontology | Use Case | MIxS Fields |
|----------|----------|-------------|
| ENVO | Environmental terms | env_broad_scale, env_local_scale, env_medium |
| UBERON | Anatomical terms | body_site, tissue |
| MONDO | Disease terms | disease, host_disease |
| NCBITaxon | Taxonomy | organism, host |

#### Exit Criteria

```bash
$ crucible analyze metadata.tsv --domain microbiome

Ontology mapping suggestions:
  → "gut" → UBERON:0000160 (intestine)
  → "stool" → UBERON:0001988 (feces)
  → "forest soil" → ENVO:00002261 (forest soil)
  → "Crohn's disease" → MONDO:0005011 (Crohn disease)
```

---

### Phase Bio-4: BioSample Pre-validator ✅ COMPLETE

**Goal**: Catch NCBI BioSample validation errors before submission.

#### Deliverables

- [x] BioSampleValidator
  - [x] Check organism/package compatibility (human packages vs organism)
  - [x] Validate sample uniqueness (attributes must differ)
  - [x] Check for null value misuse (NA, n/a, null vs NCBI-accepted values)
  - [x] Validate date formats (ISO 8601 required)
  - [x] Check geographic coordinates format (decimal degrees)
- [x] "NCBI Ready" score
  - [x] Aggregate compliance into single 0-100% metric
  - [x] Clear breakdown of blocking vs warning issues
  - [x] Integration with CLI output

#### NCBI Validation Rules

| Rule | Description |
|------|-------------|
| Mandatory fields | All required fields must have values |
| Valid organism | Must exist in NCBI Taxonomy |
| Package match | Organism must be appropriate for selected package |
| Unique samples | Attributes (excluding name/title/description) must differ |
| Valid dates | ISO 8601 or "missing" / "not collected" |
| Valid coordinates | Decimal degrees or "missing" / "not collected" |

#### Exit Criteria

```bash
$ crucible analyze metadata.tsv --domain microbiome

NCBI Readiness: 45% (NOT READY)

Blocking issues (must fix):
  ✗ Missing mandatory: lat_lon, collection_date
  ✗ Invalid organism: "E. coli" (use full name)
  ✗ Samples 3,7,12 have identical attributes

Warnings (should fix):
  ⚠ Recommended field missing: host_age
  ⚠ Non-standard format: date "Jan 15, 2024" → "2024-01-15"
```

---

### Phase Bio-5: Accession Validation (Week 3)

**Goal**: Validate format of biological database accessions.

#### Deliverables

- [ ] AccessionValidator
  - [ ] BioSample: SAMN*, SAME*, SAMD*
  - [ ] SRA: SRR*, ERR*, DRR*, SRX*, etc.
  - [ ] BioProject: PRJNA*, PRJEB*, PRJDB*
  - [ ] GenBank: Standard nucleotide accessions
  - [ ] RefSeq: NM_*, NR_*, XM_*, etc.
- [ ] Optional: API validation (check if accession exists)

---

## Data Sources

| Resource | URL | Format | Update Frequency |
|----------|-----|--------|------------------|
| MIxS Schema | https://github.com/GenomicsStandardsConsortium/mixs | JSON/LinkML | ~Yearly |
| NCBI Taxonomy | https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/ | DMP files | Daily |
| ENVO | https://github.com/EnvironmentOntology/envo | OBO/OWL | Monthly |
| UBERON | https://github.com/obophenotype/uberon | OBO/OWL | Monthly |
| MONDO | https://github.com/monarch-initiative/mondo | OBO/OWL | Monthly |

## CLI Integration

```bash
# Analyze with bioinformatics domain
crucible analyze metadata.tsv --domain microbiome
crucible analyze metadata.tsv --domain genomics
crucible analyze metadata.tsv --domain clinical

# Specify MIxS package explicitly
crucible analyze metadata.tsv --mixs-package human-gut

# Check NCBI readiness
crucible ncbi-check metadata.tsv

# Validate specific aspects
crucible analyze metadata.tsv --validate taxonomy,mixs,ontology
```

## New Observation Types

```rust
pub enum BioObservationType {
    // MIxS compliance
    MissingMandatoryField { field: String, package: String },
    MissingConditionalField { field: String, condition: String },
    InvalidFieldFormat { field: String, expected: String, actual: String },

    // Taxonomy
    InvalidTaxonomy { value: String, reason: String },
    TaxonomyTypo { value: String, suggestion: String, taxid: u32 },
    AbbreviatedTaxonomy { value: String, full_name: String, taxid: u32 },

    // Ontology
    UnmappedOntologyTerm { value: String, suggested_ontology: String },
    InvalidOntologyId { value: String, ontology: String },

    // BioSample
    DuplicateSampleAttributes { sample_ids: Vec<String> },
    OrganismPackageMismatch { organism: String, package: String },
    InvalidCoordinates { value: String },
    InvalidDateFormat { value: String, suggested: String },
}
```

## New Suggestion Actions

```rust
pub enum BioSuggestionAction {
    AddMissingField {
        field_name: String,
        requirement: RequirementLevel,  // Mandatory, Conditional, Recommended
        package: Option<String>,
        description: String,
    },
    StandardizeTaxonomy {
        input: String,
        scientific_name: String,
        taxid: u32,
        rank: String,
    },
    MapToOntology {
        input: String,
        ontology: String,       // "ENVO", "UBERON", "MONDO"
        ontology_id: String,    // "ENVO:00002261"
        label: String,          // "forest soil"
        confidence: f64,
    },
    FixDateFormat {
        input: String,
        iso_format: String,     // "2024-01-15"
    },
    FixCoordinates {
        input: String,
        lat: f64,
        lon: f64,
    },
    AddSampleDifferentiator {
        sample_ids: Vec<String>,
        suggested_attribute: String,
    },
}
```

## Success Metrics

| Metric | Target |
|--------|--------|
| MIxS validation coverage | All mandatory + conditional fields |
| Taxonomy recognition | >95% of common species names |
| Ontology term mapping | Top 500 ENVO/UBERON terms |
| NCBI rejection prevention | Catch 90% of common errors |

## Publication Plan

**Target**: *Bioinformatics* Application Note

**Title**: "Crucible: LLM-Native Metadata Curation for FAIR Bioinformatics Data"

**Key Claims**:
1. First tool to use LLM semantic understanding to infer validation rules
2. Pre-validates metadata against MIxS and NCBI BioSample requirements
3. Maps free-text to standard ontologies with LLM assistance
4. Non-destructive curation with full provenance

**Comparison**:
- vs manual spreadsheet validation: Automated, catches more errors
- vs NCBI validation: Pre-submission, immediate feedback
- vs Great Expectations: Infers rules instead of requiring specification
