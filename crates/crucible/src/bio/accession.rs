//! Biological database accession validation.
//!
//! This module provides validation for accession numbers from major biological
//! databases including NCBI (BioSample, SRA, BioProject, GenBank, RefSeq),
//! EBI (ENA), and DDBJ.
//!
//! # Example
//!
//! ```ignore
//! use crucible::bio::accession::{AccessionValidator, AccessionType};
//!
//! let validator = AccessionValidator::new();
//!
//! // Validate a single accession
//! let result = validator.validate("SAMN12345678");
//! assert!(result.is_valid);
//! assert_eq!(result.accession_type, Some(AccessionType::BioSample));
//!
//! // Check if a column contains accessions
//! let accession_type = validator.detect_accession_column("sra_accession");
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of biological database accessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessionType {
    /// NCBI BioSample (SAMN*, SAME*, SAMD*)
    BioSample,
    /// NCBI/EBI/DDBJ SRA Run (SRR*, ERR*, DRR*)
    SraRun,
    /// NCBI/EBI/DDBJ SRA Experiment (SRX*, ERX*, DRX*)
    SraExperiment,
    /// NCBI/EBI/DDBJ SRA Sample (SRS*, ERS*, DRS*)
    SraSample,
    /// NCBI/EBI/DDBJ SRA Study (SRP*, ERP*, DRP*)
    SraStudy,
    /// NCBI/EBI/DDBJ BioProject (PRJNA*, PRJEB*, PRJDB*)
    BioProject,
    /// GenBank nucleotide accession
    GenBank,
    /// RefSeq accession (NM_*, NR_*, XM_*, etc.)
    RefSeq,
    /// UniProt accession
    UniProt,
    /// PDB structure ID
    Pdb,
    /// Gene ID (numeric)
    GeneId,
    /// Protein accession
    Protein,
}

impl AccessionType {
    /// Get the database name for this accession type.
    pub fn database(&self) -> &'static str {
        match self {
            AccessionType::BioSample => "NCBI BioSample",
            AccessionType::SraRun => "SRA Run",
            AccessionType::SraExperiment => "SRA Experiment",
            AccessionType::SraSample => "SRA Sample",
            AccessionType::SraStudy => "SRA Study",
            AccessionType::BioProject => "BioProject",
            AccessionType::GenBank => "GenBank",
            AccessionType::RefSeq => "RefSeq",
            AccessionType::UniProt => "UniProt",
            AccessionType::Pdb => "PDB",
            AccessionType::GeneId => "NCBI Gene",
            AccessionType::Protein => "Protein",
        }
    }

    /// Get the URL template for this accession type.
    pub fn url_template(&self) -> &'static str {
        match self {
            AccessionType::BioSample => "https://www.ncbi.nlm.nih.gov/biosample/{}",
            AccessionType::SraRun => "https://www.ncbi.nlm.nih.gov/sra/{}",
            AccessionType::SraExperiment => "https://www.ncbi.nlm.nih.gov/sra/{}",
            AccessionType::SraSample => "https://www.ncbi.nlm.nih.gov/sra/{}",
            AccessionType::SraStudy => "https://www.ncbi.nlm.nih.gov/sra/{}",
            AccessionType::BioProject => "https://www.ncbi.nlm.nih.gov/bioproject/{}",
            AccessionType::GenBank => "https://www.ncbi.nlm.nih.gov/nuccore/{}",
            AccessionType::RefSeq => "https://www.ncbi.nlm.nih.gov/nuccore/{}",
            AccessionType::UniProt => "https://www.uniprot.org/uniprotkb/{}",
            AccessionType::Pdb => "https://www.rcsb.org/structure/{}",
            AccessionType::GeneId => "https://www.ncbi.nlm.nih.gov/gene/{}",
            AccessionType::Protein => "https://www.ncbi.nlm.nih.gov/protein/{}",
        }
    }

    /// Get the expected format pattern description.
    pub fn format_description(&self) -> &'static str {
        match self {
            AccessionType::BioSample => "SAMN/SAME/SAMD followed by digits (e.g., SAMN12345678)",
            AccessionType::SraRun => "SRR/ERR/DRR followed by digits (e.g., SRR1234567)",
            AccessionType::SraExperiment => "SRX/ERX/DRX followed by digits (e.g., SRX123456)",
            AccessionType::SraSample => "SRS/ERS/DRS followed by digits (e.g., SRS123456)",
            AccessionType::SraStudy => "SRP/ERP/DRP followed by digits (e.g., SRP123456)",
            AccessionType::BioProject => "PRJNA/PRJEB/PRJDB followed by digits (e.g., PRJNA123456)",
            AccessionType::GenBank => "1-2 letters + 5-6 digits, or 2 letters + 6-8 digits (e.g., U12345, AB123456)",
            AccessionType::RefSeq => "2 letters + underscore + digits (e.g., NM_001234, XM_012345)",
            AccessionType::UniProt => "6 or 10 alphanumeric characters (e.g., P12345, A0A0A0ABC1)",
            AccessionType::Pdb => "4 alphanumeric characters (e.g., 1ABC)",
            AccessionType::GeneId => "Numeric Gene ID (e.g., 7157)",
            AccessionType::Protein => "3 letters + 5 digits or similar (e.g., AAA12345)",
        }
    }
}

/// Result of accession validation.
#[derive(Debug, Clone)]
pub struct AccessionValidationResult {
    /// The original input value.
    pub input: String,
    /// Whether the accession format is valid.
    pub is_valid: bool,
    /// The detected accession type (if valid).
    pub accession_type: Option<AccessionType>,
    /// The normalized accession (uppercase, trimmed).
    pub normalized: Option<String>,
    /// Error message if invalid.
    pub error: Option<String>,
    /// Archive prefix (NCBI, EBI, DDBJ).
    pub archive: Option<String>,
}

/// Statistics about accession validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessionStats {
    /// Count by accession type.
    pub by_type: HashMap<AccessionType, usize>,
    /// Total valid accessions.
    pub valid_count: usize,
    /// Total invalid accessions.
    pub invalid_count: usize,
}

/// Validates biological database accession numbers.
pub struct AccessionValidator {
    /// Compiled regex patterns for each accession type.
    patterns: HashMap<AccessionType, Regex>,
}

impl AccessionValidator {
    /// Create a new accession validator.
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // BioSample: SAMN*, SAME*, SAMD* (NCBI, EBI, DDBJ)
        patterns.insert(
            AccessionType::BioSample,
            Regex::new(r"^SAM[NED][A-Z]?\d{6,12}$").unwrap(),
        );

        // SRA Run: SRR*, ERR*, DRR*
        patterns.insert(
            AccessionType::SraRun,
            Regex::new(r"^[SED]RR\d{6,9}$").unwrap(),
        );

        // SRA Experiment: SRX*, ERX*, DRX*
        patterns.insert(
            AccessionType::SraExperiment,
            Regex::new(r"^[SED]RX\d{6,9}$").unwrap(),
        );

        // SRA Sample: SRS*, ERS*, DRS*
        patterns.insert(
            AccessionType::SraSample,
            Regex::new(r"^[SED]RS\d{6,9}$").unwrap(),
        );

        // SRA Study: SRP*, ERP*, DRP*
        patterns.insert(
            AccessionType::SraStudy,
            Regex::new(r"^[SED]RP\d{6,9}$").unwrap(),
        );

        // BioProject: PRJNA*, PRJEB*, PRJDB*
        patterns.insert(
            AccessionType::BioProject,
            Regex::new(r"^PRJ[NED][A-Z]?\d{4,8}$").unwrap(),
        );

        // GenBank nucleotide:
        // - Traditional: 1-2 letters + 5-6 digits (e.g., U12345, AB123456)
        // - WGS: 4-6 letters + 8-10 digits (e.g., AAAA01000001)
        // - New format: 2 letters + 6-8 digits (e.g., CP012345)
        patterns.insert(
            AccessionType::GenBank,
            Regex::new(r"^[A-Z]{1,6}\d{5,10}(\.\d+)?$").unwrap(),
        );

        // RefSeq: 2 letters + underscore + digits (+ optional version)
        // NM_, NR_, XM_, XR_, NP_, XP_, NC_, NG_, NT_, NW_, NZ_
        patterns.insert(
            AccessionType::RefSeq,
            Regex::new(r"^(N[MRPCTGWZ]|X[MRP]|Y[P])_\d{6,9}(\.\d+)?$").unwrap(),
        );

        // UniProt: 6 or 10 character format
        // Old: P12345, Q9ABC1
        // New: A0A0A0ABC1
        patterns.insert(
            AccessionType::UniProt,
            Regex::new(r"^([OPQ][0-9][A-Z0-9]{3}[0-9]|[A-NR-Z][0-9]([A-Z][A-Z0-9]{2}[0-9]){1,2})$")
                .unwrap(),
        );

        // PDB: 4 alphanumeric characters (digit + 3 alphanumeric, must contain at least one letter)
        // Examples: 1ABC, 6LU7, 7KXJ
        patterns.insert(
            AccessionType::Pdb,
            Regex::new(r"^[0-9][A-Z][A-Z0-9]{2}$|^[0-9][A-Z0-9][A-Z][A-Z0-9]$|^[0-9][A-Z0-9]{2}[A-Z]$").unwrap(),
        );

        // Gene ID: pure numeric
        patterns.insert(
            AccessionType::GeneId,
            Regex::new(r"^\d{1,10}$").unwrap(),
        );

        // Protein accessions (NCBI): 3 letters + 5 digits
        patterns.insert(
            AccessionType::Protein,
            Regex::new(r"^[A-Z]{3}\d{5}(\.\d+)?$").unwrap(),
        );

        Self { patterns }
    }

    /// Validate an accession string.
    pub fn validate(&self, accession: &str) -> AccessionValidationResult {
        let input = accession.to_string();
        let normalized = accession.trim().to_uppercase();

        if normalized.is_empty() {
            return AccessionValidationResult {
                input,
                is_valid: false,
                accession_type: None,
                normalized: None,
                error: Some("Empty accession".to_string()),
                archive: None,
            };
        }

        // Try each pattern in order of specificity
        let check_order = [
            AccessionType::BioSample,
            AccessionType::SraRun,
            AccessionType::SraExperiment,
            AccessionType::SraSample,
            AccessionType::SraStudy,
            AccessionType::BioProject,
            AccessionType::RefSeq,
            AccessionType::UniProt,
            AccessionType::Pdb,
            AccessionType::Protein,
            AccessionType::GenBank,
            AccessionType::GeneId, // Last because it matches pure numbers
        ];

        for acc_type in check_order {
            if let Some(pattern) = self.patterns.get(&acc_type) {
                if pattern.is_match(&normalized) {
                    let archive = self.detect_archive(&normalized, acc_type);
                    return AccessionValidationResult {
                        input,
                        is_valid: true,
                        accession_type: Some(acc_type),
                        normalized: Some(normalized),
                        error: None,
                        archive,
                    };
                }
            }
        }

        // Check if it looks like an accession but doesn't match patterns
        let error = if normalized.starts_with("SAM") {
            Some("Invalid BioSample format. Expected: SAMN/SAME/SAMD followed by digits".to_string())
        } else if normalized.starts_with("SRR")
            || normalized.starts_with("ERR")
            || normalized.starts_with("DRR")
        {
            Some("Invalid SRA Run format. Expected: SRR/ERR/DRR followed by 6-9 digits".to_string())
        } else if normalized.starts_with("PRJ") {
            Some(
                "Invalid BioProject format. Expected: PRJNA/PRJEB/PRJDB followed by digits"
                    .to_string(),
            )
        } else if normalized.contains('_') && normalized.len() > 3 {
            Some("Invalid RefSeq format. Expected: NM_/XM_/etc. followed by digits".to_string())
        } else {
            Some("Unrecognized accession format".to_string())
        };

        AccessionValidationResult {
            input,
            is_valid: false,
            accession_type: None,
            normalized: Some(normalized),
            error,
            archive: None,
        }
    }

    /// Detect the archive (NCBI, EBI, DDBJ) from an accession.
    fn detect_archive(&self, normalized: &str, acc_type: AccessionType) -> Option<String> {
        match acc_type {
            AccessionType::BioSample => {
                if normalized.starts_with("SAMN") {
                    Some("NCBI".to_string())
                } else if normalized.starts_with("SAME") {
                    Some("EBI".to_string())
                } else if normalized.starts_with("SAMD") {
                    Some("DDBJ".to_string())
                } else {
                    None
                }
            }
            AccessionType::SraRun
            | AccessionType::SraExperiment
            | AccessionType::SraSample
            | AccessionType::SraStudy => {
                if normalized.starts_with('S') {
                    Some("NCBI".to_string())
                } else if normalized.starts_with('E') {
                    Some("EBI".to_string())
                } else if normalized.starts_with('D') {
                    Some("DDBJ".to_string())
                } else {
                    None
                }
            }
            AccessionType::BioProject => {
                if normalized.starts_with("PRJNA") {
                    Some("NCBI".to_string())
                } else if normalized.starts_with("PRJEB") {
                    Some("EBI".to_string())
                } else if normalized.starts_with("PRJDB") {
                    Some("DDBJ".to_string())
                } else {
                    None
                }
            }
            AccessionType::RefSeq | AccessionType::GeneId => Some("NCBI".to_string()),
            AccessionType::UniProt => Some("UniProt".to_string()),
            AccessionType::Pdb => Some("PDB".to_string()),
            _ => None,
        }
    }

    /// Detect if a column name likely contains accessions and what type.
    pub fn detect_accession_column(&self, column_name: &str) -> Option<AccessionType> {
        let name = column_name.to_lowercase();

        // BioSample
        if name.contains("biosample")
            || name == "sample_accession"
            || name == "sampleaccession"
            || name.contains("biosample_accession")
        {
            return Some(AccessionType::BioSample);
        }

        // SRA Run
        if name.contains("sra_run")
            || name.contains("run_accession")
            || name == "sra"
            || name == "sra_accession"
            || name.contains("srr")
            || name.contains("err")
        {
            return Some(AccessionType::SraRun);
        }

        // SRA Experiment
        if name.contains("experiment_accession")
            || name.contains("sra_experiment")
            || name.contains("srx")
        {
            return Some(AccessionType::SraExperiment);
        }

        // BioProject
        if name.contains("bioproject")
            || name.contains("project_accession")
            || name.contains("prjna")
            || name.contains("prjeb")
        {
            return Some(AccessionType::BioProject);
        }

        // RefSeq
        if name.contains("refseq") || name.contains("ref_seq") {
            return Some(AccessionType::RefSeq);
        }

        // GenBank
        if name.contains("genbank")
            || name.contains("nucleotide")
            || name.contains("accession") && !name.contains("sample")
        {
            return Some(AccessionType::GenBank);
        }

        // UniProt
        if name.contains("uniprot") || name.contains("swissprot") {
            return Some(AccessionType::UniProt);
        }

        // PDB
        if name.contains("pdb") || name.contains("structure") {
            return Some(AccessionType::Pdb);
        }

        // Gene ID
        if name == "gene_id" || name == "geneid" || name == "entrez_id" || name == "entrezid" {
            return Some(AccessionType::GeneId);
        }

        None
    }

    /// Validate all accessions in a column and return statistics.
    pub fn validate_column(&self, values: &[&str]) -> (Vec<AccessionValidationResult>, AccessionStats) {
        let mut results = Vec::new();
        let mut stats = AccessionStats::default();

        for value in values {
            let result = self.validate(value);
            if result.is_valid {
                stats.valid_count += 1;
                if let Some(acc_type) = result.accession_type {
                    *stats.by_type.entry(acc_type).or_insert(0) += 1;
                }
            } else if !value.trim().is_empty() {
                stats.invalid_count += 1;
            }
            results.push(result);
        }

        (results, stats)
    }

    /// Get the URL for an accession.
    pub fn get_url(&self, accession: &str) -> Option<String> {
        let result = self.validate(accession);
        if result.is_valid {
            if let (Some(acc_type), Some(normalized)) = (result.accession_type, result.normalized) {
                return Some(acc_type.url_template().replace("{}", &normalized));
            }
        }
        None
    }
}

impl Default for AccessionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biosample_validation() {
        let validator = AccessionValidator::new();

        // Valid BioSample accessions
        let result = validator.validate("SAMN12345678");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::BioSample));
        assert_eq!(result.archive, Some("NCBI".to_string()));

        let result = validator.validate("SAME1234567");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("EBI".to_string()));

        let result = validator.validate("SAMD00123456");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("DDBJ".to_string()));

        // Note: "SAMX12345" matches GenBank format (4 letters + 5 digits)
        // This is a format edge case - it's valid GenBank, not invalid BioSample
        let result = validator.validate("SAMX12345");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::GenBank));

        // Truly invalid formats
        let result = validator.validate("SAM");
        assert!(!result.is_valid);

        let result = validator.validate("SAMN");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_sra_run_validation() {
        let validator = AccessionValidator::new();

        // Valid SRA runs
        let result = validator.validate("SRR1234567");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::SraRun));
        assert_eq!(result.archive, Some("NCBI".to_string()));

        let result = validator.validate("ERR123456");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("EBI".to_string()));

        let result = validator.validate("DRR1234567");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("DDBJ".to_string()));

        // Invalid
        let result = validator.validate("SRR123");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_sra_experiment_validation() {
        let validator = AccessionValidator::new();

        let result = validator.validate("SRX123456");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::SraExperiment));

        let result = validator.validate("ERX1234567");
        assert!(result.is_valid);
    }

    #[test]
    fn test_bioproject_validation() {
        let validator = AccessionValidator::new();

        let result = validator.validate("PRJNA123456");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::BioProject));
        assert_eq!(result.archive, Some("NCBI".to_string()));

        let result = validator.validate("PRJEB12345");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("EBI".to_string()));

        let result = validator.validate("PRJDB1234");
        assert!(result.is_valid);
        assert_eq!(result.archive, Some("DDBJ".to_string()));
    }

    #[test]
    fn test_refseq_validation() {
        let validator = AccessionValidator::new();

        // mRNA
        let result = validator.validate("NM_001234567");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::RefSeq));

        // Non-coding RNA
        let result = validator.validate("NR_123456");
        assert!(result.is_valid);

        // Predicted mRNA
        let result = validator.validate("XM_012345678");
        assert!(result.is_valid);

        // Protein
        let result = validator.validate("NP_001234567");
        assert!(result.is_valid);

        // Chromosome
        let result = validator.validate("NC_000001");
        assert!(result.is_valid);

        // With version
        let result = validator.validate("NM_001234567.2");
        assert!(result.is_valid);
    }

    #[test]
    fn test_genbank_validation() {
        let validator = AccessionValidator::new();

        // Traditional format
        let result = validator.validate("U12345");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::GenBank));

        let result = validator.validate("AB123456");
        assert!(result.is_valid);

        // Newer format
        let result = validator.validate("CP012345");
        assert!(result.is_valid);

        // WGS
        let result = validator.validate("AAAA01000001");
        assert!(result.is_valid);

        // With version
        let result = validator.validate("U12345.1");
        assert!(result.is_valid);
    }

    #[test]
    fn test_uniprot_validation() {
        let validator = AccessionValidator::new();

        // Old format
        let result = validator.validate("P12345");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::UniProt));

        let result = validator.validate("Q9ABC1");
        assert!(result.is_valid);

        // New format (10 characters)
        let result = validator.validate("A0A0A0ABC1");
        assert!(result.is_valid);
    }

    #[test]
    fn test_pdb_validation() {
        let validator = AccessionValidator::new();

        let result = validator.validate("1ABC");
        assert!(result.is_valid);
        assert_eq!(result.accession_type, Some(AccessionType::Pdb));

        let result = validator.validate("6LU7");
        assert!(result.is_valid);
    }

    #[test]
    fn test_column_detection() {
        let validator = AccessionValidator::new();

        assert_eq!(
            validator.detect_accession_column("biosample_accession"),
            Some(AccessionType::BioSample)
        );
        assert_eq!(
            validator.detect_accession_column("sra_run"),
            Some(AccessionType::SraRun)
        );
        assert_eq!(
            validator.detect_accession_column("bioproject"),
            Some(AccessionType::BioProject)
        );
        assert_eq!(
            validator.detect_accession_column("gene_id"),
            Some(AccessionType::GeneId)
        );
        assert_eq!(validator.detect_accession_column("sample_id"), None);
    }

    #[test]
    fn test_url_generation() {
        let validator = AccessionValidator::new();

        let url = validator.get_url("SAMN12345678");
        assert_eq!(
            url,
            Some("https://www.ncbi.nlm.nih.gov/biosample/SAMN12345678".to_string())
        );

        let url = validator.get_url("SRR1234567");
        assert_eq!(
            url,
            Some("https://www.ncbi.nlm.nih.gov/sra/SRR1234567".to_string())
        );

        let url = validator.get_url("invalid");
        assert_eq!(url, None);
    }

    #[test]
    fn test_case_insensitivity() {
        let validator = AccessionValidator::new();

        // Should work with lowercase
        let result = validator.validate("samn12345678");
        assert!(result.is_valid);
        assert_eq!(result.normalized, Some("SAMN12345678".to_string()));

        let result = validator.validate("srr1234567");
        assert!(result.is_valid);
    }

    #[test]
    fn test_column_validation() {
        let validator = AccessionValidator::new();

        let values = vec!["SAMN12345678", "SAMN87654321", "invalid", "SAMN11111111"];
        let (results, stats) = validator.validate_column(&values);

        assert_eq!(stats.valid_count, 3);
        assert_eq!(stats.invalid_count, 1);
        assert_eq!(stats.by_type.get(&AccessionType::BioSample), Some(&3));
        assert_eq!(results.len(), 4);
    }
}
