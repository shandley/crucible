# Crucible

**Intelligent data curation for tabular datasets.**

Crucible analyzes your CSV/TSV files, detects data quality issues, and suggests fixes—all without modifying your original data. An AI-powered curation layer tracks every observation, suggestion, and decision with full provenance.

## What Crucible Does

- **Detects issues**: Missing values, outliers, inconsistent formatting, case variations, type mismatches
- **Suggests fixes**: Standardize values, fill missing data, flag anomalies for review
- **Tracks everything**: Every change is recorded with who made it and why
- **Preserves originals**: Your source data is never modified; curations are stored separately

## Installation

### Prerequisites

- **Rust** (1.70 or later): Install from [rustup.rs](https://rustup.rs/)
- **Node.js** (18 or later): Required for building the web UI

### From Source

```bash
# Clone the repository
git clone https://github.com/shandley/crucible.git
cd crucible

# Build and install
cargo install --path crates/crucible-cli

# Verify installation
crucible --help
```

### Optional: Enable Parquet Support

```bash
cargo install --path crates/crucible-cli --features parquet
```

## Quick Start

```bash
# Analyze a data file
crucible analyze data.tsv

# Open the interactive web UI to review suggestions
crucible review data.tsv

# Apply accepted changes and export
crucible apply data.curation.json -o curated.tsv
```

## Usage Guide

### Analyzing Data

The `analyze` command scans your data and creates a curation file with detected issues and suggested fixes:

```bash
crucible analyze data.tsv
```

This creates `data.curation.json` containing:
- Inferred schema (column types, semantic roles)
- Observations (detected issues)
- Suggestions (proposed fixes)
- Data quality score

**Options:**

```bash
# Specify output file
crucible analyze data.tsv --output my-curation.json

# Provide domain context for better suggestions
crucible analyze data.tsv --domain biomedical

# Skip LLM enhancement (faster, works offline)
crucible analyze data.tsv --no-llm
```

### Interactive Review (Web UI)

The `review` command starts a local web server with an interactive UI:

```bash
crucible review data.tsv
```

This opens your browser to `http://localhost:3141` where you can:

- **Review suggestions** one by one with Accept/Reject/Modify buttons
- **See affected data** with highlighted rows showing what will change
- **Use keyboard shortcuts** for faster review:
  - `j`/`k` - Navigate between suggestions
  - `a` - Accept current suggestion
  - `r` - Reject current suggestion
  - `Enter` - Expand/collapse suggestion details
  - `Ctrl+Z` - Undo last decision
- **Batch operations** - Accept or reject all suggestions for a column
- **Ask AI questions** about observations (requires API key)
- **Toast notifications** for save confirmations and errors
- **AI status indicator** showing if LLM features are available

**Options:**

```bash
# Use a different port
crucible review data.tsv --port 8080

# Don't auto-open browser
crucible review data.tsv --no-open

# Review an existing curation file
crucible review data.tsv --curation existing.curation.json
```

### Checking Progress

View curation progress without opening the web UI:

```bash
crucible status data.curation.json
```

Output:
```
Curation Status: data.curation.json

Progress: 15/22 suggestions reviewed (68%)
  Accepted: 12
  Rejected: 2
  Modified: 1
  Pending:  7

Data Quality: 85% (Good)
```

**Options:**

```bash
# JSON output for scripting
crucible status data.curation.json --json
```

### Previewing Changes

See what changes would be applied before committing:

```bash
crucible diff data.curation.json
```

Output:
```
Changes to be applied (12 accepted suggestions):

Column: diagnosis
  Row 5: "Crohn's" → "CD" (standardize)
  Row 12: "cd" → "CD" (standardize)

Column: age
  Row 8: "" → "NA" (fill missing)

Column: bmi
  Row 15: Flagged as outlier (value: -5.2)
```

**Options:**

```bash
# Show more context lines
crucible diff data.curation.json --context 5
```

### Applying Changes

Export curated data with accepted changes applied:

```bash
crucible apply data.curation.json -o curated.tsv
```

**Supported formats:**

```bash
# Tab-separated (default)
crucible apply data.curation.json -o curated.tsv --format tsv

# Comma-separated
crucible apply data.curation.json -o curated.csv --format csv

# JSON (array of objects)
crucible apply data.curation.json -o curated.json --format json

# Parquet (requires --features parquet)
crucible apply data.curation.json -o curated.parquet --format parquet
```

### Batch Operations

Accept or reject multiple suggestions at once:

```bash
# Accept all standardization suggestions
crucible batch data.curation.json --accept --action-type standardize

# Reject all suggestions for a specific column
crucible batch data.curation.json --reject --column diagnosis

# Accept all remaining pending suggestions
crucible batch data.curation.json --accept --all
```

## AI Features

Crucible can use AI to enhance analysis and provide interactive explanations.

### Setting Up AI

Set one of these environment variables:

```bash
# Anthropic Claude (recommended)
export ANTHROPIC_API_KEY="your-api-key"

# OpenAI
export OPENAI_API_KEY="your-api-key"

# Local Ollama (no API key needed)
# Just ensure Ollama is running: ollama serve
```

### What AI Enables

When an API key is configured:

1. **Enhanced Analysis**: Better detection of domain-specific issues
2. **Ask Questions**: Click "Ask" on any observation to get AI explanations
3. **Confidence Calibration**: AI adjusts confidence scores based on context

The web UI shows AI status in the header. The "Ask" button only appears when AI is available.

### Running Without AI

Crucible works fully offline without any API keys. AI features are simply disabled:

```bash
# Explicitly skip AI during analysis
crucible analyze data.tsv --no-llm
```

## Examples

### Biomedical Metadata

```bash
# Analyze with biomedical domain context
crucible analyze patient_metadata.tsv --domain biomedical

# Review interactively
crucible review patient_metadata.tsv

# After review, export cleaned data
crucible apply patient_metadata.curation.json -o cleaned_metadata.tsv
```

### Quick Validation

```bash
# Just check data quality score
crucible analyze data.csv
crucible status data.curation.json --json | jq '.quality_score'
```

### CI/CD Integration

```bash
# Fail if data quality is below threshold
SCORE=$(crucible status data.curation.json --json | jq '.quality_score')
if (( $(echo "$SCORE < 0.8" | bc -l) )); then
  echo "Data quality too low: $SCORE"
  exit 1
fi
```

## File Formats

### Input

Crucible automatically detects:
- **TSV** (tab-separated)
- **CSV** (comma-separated)
- **Semicolon-separated**
- **Pipe-separated**

### Curation Layer

The `.curation.json` file stores all analysis results and decisions:

```json
{
  "version": "1.0.0",
  "source_file": "data.tsv",
  "schema": { ... },
  "observations": [ ... ],
  "suggestions": [ ... ],
  "decisions": [ ... ],
  "summary": {
    "data_quality_score": 0.85,
    "recommendation": "Good quality with minor issues"
  }
}
```

This file can be:
- Version controlled alongside your data
- Shared with collaborators
- Used to reproduce exact curation decisions

## Troubleshooting

### "Command not found: crucible"

Ensure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Web UI won't open

Try specifying a different port:

```bash
crucible review data.tsv --port 8080 --no-open
# Then manually open http://localhost:8080
```

### AI features not working

Check that your API key is set:

```bash
echo $ANTHROPIC_API_KEY  # Should show your key
```

The web UI header shows "AI: Enabled" or "AI: Disabled" to confirm status.

### Large files

Crucible is optimized for files up to 100MB (~500K rows):

- **100K rows analyzed in ~2 seconds**
- Virtual scrolling for smooth navigation through large datasets
- Pagination API fetches data on-demand

For very large files, skip AI enhancement for faster analysis:

```bash
crucible analyze large_data.tsv --no-llm
```

## Getting Help

```bash
# General help
crucible --help

# Command-specific help
crucible analyze --help
crucible review --help
crucible apply --help
```

## License

MIT License - see [LICENSE](./LICENSE) for details.
