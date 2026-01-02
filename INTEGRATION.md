# Crucible Integration with BioStack

This document describes how Crucible integrates with the BioStack ecosystem.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          USER WORKFLOW                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. Extract          2. Curate           3. Analyze        4. Publish  │
│  ┌──────────┐       ┌──────────┐        ┌──────────┐      ┌──────────┐ │
│  │ biostack │       │ biostack │        │ biostack │      │ biostack │ │
│  │ -extract │  ──►  │ -curate  │  ──►   │          │  ──► │-publish  │ │
│  └──────────┘       └──────────┘        └──────────┘      └──────────┘ │
│       │                  │                   │                  │       │
│       ▼                  ▼                   ▼                  ▼       │
│  ┌──────────┐       ┌──────────┐        ┌──────────┐      ┌──────────┐ │
│  │ phyloseq │       │ crucible │        │ bioforge │      │ methods  │ │
│  │ extractor│       │   core   │        │primitives│      │ section  │ │
│  └──────────┘       └──────────┘        └──────────┘      └──────────┘ │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Integration Points

### 1. biostack-curate Skill

A Claude Code skill that wraps Crucible with biological context.

```
.claude/skills/biostack-curate/
├── SKILL.md           # When to use, workflow
├── BIOLOGICAL_HINTS.md # Domain-specific context for LLM
└── PROMPTS.md         # Custom prompts for biological data
```

**SKILL.md** (excerpt):
```yaml
---
name: biostack-curate
description: Curate biological metadata using Crucible with domain-specific context. Use when preparing metadata for analysis or when data quality issues are suspected.
---

# BioStack Curate Skill

Curate sample metadata with biological awareness.

## When to Use

- After extracting data from phyloseq/Seurat/AnnData
- When diagnostics report sample mismatches
- Before running statistical analyses
- When metadata has known quality issues

## Workflow

1. Run Crucible with biological context
2. Review AI suggestions
3. Accept/reject curations
4. Export curated metadata
5. Re-run diagnostics to verify
```

### 2. Biological Context Hints

The biostack-curate skill provides domain-specific hints to Crucible:

```json
{
  "context": {
    "hints": {
      "domain": "microbiome",
      "data_type": "16S_amplicon",
      "common_variables": {
        "diagnosis": {
          "description": "Disease diagnosis",
          "expected_values": ["CD", "UC", "IC", "healthy", "control"],
          "ontology": "MONDO"
        },
        "age": {
          "description": "Subject age at sample collection",
          "expected_range": [0, 120],
          "pediatric_range": [0, 18]
        },
        "sex": {
          "description": "Biological sex",
          "expected_values": ["male", "female", "M", "F"],
          "standardize_to": ["male", "female"]
        },
        "biopsy_location": {
          "description": "GI tract sampling location",
          "expected_values": ["ileum", "colon", "rectum", "cecum", "duodenum"]
        }
      },
      "identifier_patterns": {
        "sample_id": "^[A-Z0-9._-]+$",
        "subject_id": "^[A-Z]{1,3}-?\\d+$"
      }
    }
  }
}
```

### 3. LLM Prompt Enhancement

biostack-curate provides biological context to LLM prompts:

**Standard Crucible prompt**:
```
Analyze this column and infer its type and constraints:
Column: diagnosis
Values: CD, UC, IC, no, Control, control
```

**biostack-curate enhanced prompt**:
```
Analyze this column from a microbiome study metadata file.

Column: diagnosis
Values: CD, UC, IC, no, Control, control

Biological context:
- This is likely an IBD (inflammatory bowel disease) study
- CD = Crohn's Disease, UC = Ulcerative Colitis, IC = Indeterminate Colitis
- "no" and "control" likely indicate healthy controls
- Case variations should be standardized

Infer the column type, expected values, and any issues.
```

### 4. Curation Layer in BioStack Provenance

Crucible's curation layer integrates with BioStack's provenance tracking:

```
.biostack/
├── data/
│   ├── counts.tsv
│   ├── metadata.tsv              # Original metadata
│   └── metadata.curation.json    # Crucible curation layer
├── curated/
│   └── metadata.tsv              # Curated metadata (if exported)
└── provenance/
    └── session.db                # Records curation as operation
```

**Provenance integration**:
```json
{
  "operation": "curate_metadata",
  "tool": "crucible",
  "version": "1.0.0",
  "input": {
    "file": "metadata.tsv",
    "hash": "sha256:abc123..."
  },
  "output": {
    "curation_layer": "metadata.curation.json",
    "curated_file": "curated/metadata.tsv"
  },
  "decisions": {
    "accepted": 15,
    "rejected": 3,
    "modified": 2
  },
  "timestamp": "2024-12-30T12:00:00Z"
}
```

### 5. Diagnostics Integration

After curation, BioStack diagnostics automatically picks up the curated metadata:

```
┌─────────────────────┐
│ Original metadata   │
│ metadata.tsv        │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Crucible curation   │
│ (biostack-curate)   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Curated metadata    │
│ curated/metadata.tsv│
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ BioStack diagnostics│
│ (daemon auto-detects│
│  curated file)      │
└─────────────────────┘
```

### 6. Publication Integration

biostack-publication includes curation in methods section:

**Generated methods text**:
```
Sample metadata was curated using Crucible v1.0.0. Automated quality
assessment identified 47 potential issues across 73 variables. After
review, 15 curations were applied: standardization of categorical
variables (n=8), conversion of missing value representations (n=5),
and outlier flagging (n=2). Full curation provenance is available in
the reproducibility package.
```

## Workflow Examples

### Example 1: Post-Extraction Curation

```bash
# 1. Extract from phyloseq
Rscript EXTRACTORS/phyloseq.R data.rds .

# 2. Curate metadata (via skill or CLI)
crucible analyze metadata.tsv \
  --context-file biostack_context.json \
  -o metadata.curation.json

# 3. Review and decide
crucible status metadata.curation.json
# Accept high-confidence suggestions
crucible accept metadata.curation.json --confidence-above 0.9

# 4. Export curated metadata
crucible apply metadata.curation.json -o curated/metadata.tsv

# 5. Restart diagnostics (daemon auto-detects)
bio start --daemon-only
```

### Example 2: Interactive Curation (via Skill)

When user says: "Clean up my metadata"

**biostack-curate skill**:
1. Reads current metadata from session
2. Runs Crucible with biological context
3. Presents observations to user
4. Collects accept/reject decisions
5. Exports curated metadata
6. Updates BioStack provenance

### Example 3: Pre-Analysis Validation

```yaml
# Before running PERMANOVA, validate metadata
primitive: validate_metadata
inputs:
  metadata: metadata.tsv
params:
  crucible_confidence: 0.8
  require_clean: true  # Fail if high-severity issues exist
```

## API Integration

### Rust (Direct)

```rust
use crucible::{Crucible, ContextHints};
use biostack_curate::BiologicalContext;

// Load biological context
let bio_context = BiologicalContext::microbiome();

// Create crucible with enhanced context
let crucible = Crucible::new()
    .with_llm(provider)
    .with_context(bio_context.as_hints());

let curation = crucible.analyze(metadata_path)?;
```

### Python (via PyO3)

```python
from crucible import Crucible
from biostack_curate import BiologicalContext

# Load biological context
bio_context = BiologicalContext.microbiome()

# Create crucible with enhanced context
crucible = Crucible(
    llm="anthropic",
    context=bio_context.as_hints()
)

curation = crucible.analyze("metadata.tsv")
```

## Extension Points

### Custom Biological Hints

Users can extend biological context:

```json
{
  "custom_variables": {
    "my_custom_score": {
      "description": "Custom disease activity score",
      "expected_range": [0, 10],
      "higher_is_worse": true
    }
  }
}
```

### Domain-Specific Validators

biostack-curate can register additional validators:

```rust
// Validate taxonomy strings
crucible.register_validator(TaxonomyValidator::new());

// Validate phylogenetic constraints
crucible.register_validator(PhylogeneticConsistencyValidator::new());
```

### Ontology Integration

Map categorical variables to ontologies:

```json
{
  "ontology_mappings": {
    "diagnosis": {
      "CD": "MONDO:0005011",
      "UC": "MONDO:0005101"
    },
    "biopsy_location": {
      "ileum": "UBERON:0002116",
      "colon": "UBERON:0001155"
    }
  }
}
```

## Migration Path

For existing BioStack users:

1. **Phase 1**: Crucible available as standalone tool
2. **Phase 2**: biostack-curate skill wraps Crucible
3. **Phase 3**: Integration with diagnostics daemon
4. **Phase 4**: Full provenance integration

Each phase is usable independently; users can adopt incrementally.
