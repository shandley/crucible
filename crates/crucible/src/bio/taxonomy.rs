//! NCBI Taxonomy validation.
//!
//! This module provides validation of taxonomic names against the NCBI Taxonomy
//! database, including detection of common abbreviations, typos, and suggestions
//! for standardization.
//!
//! # Loading Taxonomy Data
//!
//! The validator can load taxonomy data from:
//! 1. Built-in database of ~150 common organisms
//! 2. NCBI Taxonomy dump files (names.dmp, nodes.dmp)
//!
//! ```ignore
//! use crucible::bio::TaxonomyValidator;
//!
//! // Use built-in database
//! let validator = TaxonomyValidator::new();
//!
//! // Or load from NCBI dump files
//! let validator = TaxonomyValidator::from_ncbi_dump("taxdump/names.dmp", "taxdump/nodes.dmp")?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A taxonomy entry from NCBI Taxonomy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyEntry {
    /// NCBI Taxonomy ID.
    pub taxid: u32,
    /// Scientific name.
    pub scientific_name: String,
    /// Taxonomic rank (e.g., species, genus, family).
    pub rank: String,
    /// Common names (if any).
    pub common_names: Vec<String>,
    /// Parent taxonomy ID (for lineage lookups).
    #[serde(default)]
    pub parent_taxid: Option<u32>,
}

/// Statistics about loaded taxonomy data.
#[derive(Debug, Clone, Default)]
pub struct TaxonomyStats {
    /// Total number of taxa loaded.
    pub total_taxa: usize,
    /// Number of species.
    pub species_count: usize,
    /// Number of genera.
    pub genus_count: usize,
    /// Number of abbreviations.
    pub abbreviation_count: usize,
    /// Data source description.
    pub source: String,
}

/// Taxonomy validator for checking organism names.
#[derive(Debug, Clone)]
pub struct TaxonomyValidator {
    /// Common abbreviations and their expansions.
    abbreviations: HashMap<String, String>,
    /// Organism lookup by lowercase name.
    known_organisms: HashMap<String, TaxonomyEntry>,
    /// Taxid to entry lookup.
    taxid_index: HashMap<u32, TaxonomyEntry>,
    /// Statistics about loaded data.
    stats: TaxonomyStats,
}

impl TaxonomyValidator {
    /// Create a new taxonomy validator with built-in common organisms.
    pub fn new() -> Self {
        let mut validator = Self {
            abbreviations: HashMap::new(),
            known_organisms: HashMap::new(),
            taxid_index: HashMap::new(),
            stats: TaxonomyStats {
                source: "built-in".to_string(),
                ..Default::default()
            },
        };
        validator.load_common_abbreviations();
        validator.load_common_organisms();
        validator.update_stats();
        validator
    }

    /// Create a validator from NCBI Taxonomy dump files.
    ///
    /// Downloads available from: https://ftp.ncbi.nlm.nih.gov/pub/taxonomy/taxdump.tar.gz
    ///
    /// # Arguments
    /// * `names_path` - Path to names.dmp file
    /// * `nodes_path` - Path to nodes.dmp file (optional, for rank info)
    pub fn from_ncbi_dump(
        names_path: impl AsRef<Path>,
        nodes_path: Option<impl AsRef<Path>>,
    ) -> Result<Self, std::io::Error> {
        let mut validator = Self {
            abbreviations: HashMap::new(),
            known_organisms: HashMap::new(),
            taxid_index: HashMap::new(),
            stats: TaxonomyStats {
                source: format!("NCBI dump: {}", names_path.as_ref().display()),
                ..Default::default()
            },
        };

        // Load ranks from nodes.dmp if provided
        let ranks: HashMap<u32, (String, u32)> = if let Some(nodes) = nodes_path {
            validator.parse_nodes_dmp(nodes)?
        } else {
            HashMap::new()
        };

        // Load names from names.dmp
        validator.parse_names_dmp(names_path, &ranks)?;

        // Load abbreviations
        validator.load_common_abbreviations();
        validator.update_stats();

        Ok(validator)
    }

    /// Parse nodes.dmp file to get rank and parent information.
    fn parse_nodes_dmp(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<HashMap<u32, (String, u32)>, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut ranks = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split("\t|\t").collect();
            if parts.len() >= 3 {
                if let Ok(taxid) = parts[0].trim_end_matches("\t|").parse::<u32>() {
                    let parent_taxid = parts[1].parse::<u32>().unwrap_or(0);
                    let rank = parts[2].trim().to_string();
                    ranks.insert(taxid, (rank, parent_taxid));
                }
            }
        }

        Ok(ranks)
    }

    /// Parse names.dmp file to load taxonomy entries.
    fn parse_names_dmp(
        &mut self,
        path: impl AsRef<Path>,
        ranks: &HashMap<u32, (String, u32)>,
    ) -> Result<(), std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Temporary storage for building entries
        let mut scientific_names: HashMap<u32, String> = HashMap::new();
        let mut common_names: HashMap<u32, Vec<String>> = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split("\t|\t").collect();
            if parts.len() >= 4 {
                if let Ok(taxid) = parts[0].trim_end_matches("\t|").parse::<u32>() {
                    let name = parts[1].trim().to_string();
                    let name_class = parts[3].trim_end_matches("\t|").trim();

                    match name_class {
                        "scientific name" => {
                            scientific_names.insert(taxid, name);
                        }
                        "common name" | "genbank common name" => {
                            common_names.entry(taxid).or_default().push(name);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Build entries from collected data
        for (taxid, scientific_name) in scientific_names {
            let (rank, parent_taxid) = ranks
                .get(&taxid)
                .cloned()
                .unwrap_or(("no rank".to_string(), 0));

            let entry = TaxonomyEntry {
                taxid,
                scientific_name: scientific_name.clone(),
                rank,
                common_names: common_names.remove(&taxid).unwrap_or_default(),
                parent_taxid: if parent_taxid > 0 {
                    Some(parent_taxid)
                } else {
                    None
                },
            };

            // Index by scientific name
            self.known_organisms
                .insert(scientific_name.to_lowercase(), entry.clone());

            // Index by common names
            for common in &entry.common_names {
                self.known_organisms
                    .insert(common.to_lowercase(), entry.clone());
            }

            // Index by taxid
            self.taxid_index.insert(taxid, entry);
        }

        Ok(())
    }

    /// Update statistics after loading data.
    fn update_stats(&mut self) {
        self.stats.total_taxa = self.taxid_index.len();
        self.stats.abbreviation_count = self.abbreviations.len();
        self.stats.species_count = self
            .taxid_index
            .values()
            .filter(|e| e.rank == "species")
            .count();
        self.stats.genus_count = self
            .taxid_index
            .values()
            .filter(|e| e.rank == "genus")
            .count();
    }

    /// Get statistics about loaded taxonomy data.
    pub fn stats(&self) -> &TaxonomyStats {
        &self.stats
    }

    /// Look up an organism by taxonomy ID.
    pub fn lookup_by_taxid(&self, taxid: u32) -> Option<&TaxonomyEntry> {
        self.taxid_index.get(&taxid)
    }

    /// Get the number of known organisms.
    pub fn organism_count(&self) -> usize {
        self.taxid_index.len()
    }

    /// Load common taxonomic abbreviations.
    fn load_common_abbreviations(&mut self) {
        let abbrevs = [
            ("E. coli", "Escherichia coli"),
            ("S. aureus", "Staphylococcus aureus"),
            ("B. subtilis", "Bacillus subtilis"),
            ("P. aeruginosa", "Pseudomonas aeruginosa"),
            ("S. cerevisiae", "Saccharomyces cerevisiae"),
            ("C. elegans", "Caenorhabditis elegans"),
            ("D. melanogaster", "Drosophila melanogaster"),
            ("M. musculus", "Mus musculus"),
            ("H. sapiens", "Homo sapiens"),
            ("A. thaliana", "Arabidopsis thaliana"),
            ("B. fragilis", "Bacteroides fragilis"),
            ("C. difficile", "Clostridioides difficile"),
            ("F. prausnitzii", "Faecalibacterium prausnitzii"),
            ("L. reuteri", "Limosilactobacillus reuteri"),
            ("B. longum", "Bifidobacterium longum"),
        ];

        for (abbrev, full) in abbrevs {
            self.abbreviations
                .insert(abbrev.to_lowercase(), full.to_string());
            // Also add without period
            let no_period = abbrev.replace('.', "");
            self.abbreviations
                .insert(no_period.to_lowercase(), full.to_string());
        }
    }

    /// Load common organisms for quick validation.
    ///
    /// This includes ~150 commonly encountered organisms in microbiome,
    /// genomics, and biomedical research.
    fn load_common_organisms(&mut self) {
        // Format: (taxid, scientific_name, rank, common_names)
        let organisms: &[(u32, &str, &str, &[&str])] = &[
            // === Model Organisms ===
            (9606, "Homo sapiens", "species", &["human"]),
            (10090, "Mus musculus", "species", &["mouse", "house mouse"]),
            (10116, "Rattus norvegicus", "species", &["rat", "brown rat"]),
            (9031, "Gallus gallus", "species", &["chicken"]),
            (7955, "Danio rerio", "species", &["zebrafish"]),
            (8364, "Xenopus tropicalis", "species", &["western clawed frog"]),
            (7227, "Drosophila melanogaster", "species", &["fruit fly"]),
            (6239, "Caenorhabditis elegans", "species", &["C. elegans", "roundworm"]),
            (4932, "Saccharomyces cerevisiae", "species", &["yeast", "baker's yeast", "budding yeast"]),
            (4896, "Schizosaccharomyces pombe", "species", &["fission yeast"]),
            (3702, "Arabidopsis thaliana", "species", &["thale cress"]),

            // === Common Bacteria (Gram-negative) ===
            (562, "Escherichia coli", "species", &["E. coli"]),
            (287, "Pseudomonas aeruginosa", "species", &["P. aeruginosa"]),
            (573, "Klebsiella pneumoniae", "species", &["K. pneumoniae"]),
            (550, "Enterobacter cloacae", "species", &["E. cloacae"]),
            (485, "Neisseria meningitidis", "species", &["meningococcus"]),
            (487, "Neisseria gonorrhoeae", "species", &["gonococcus"]),
            (210, "Helicobacter pylori", "species", &["H. pylori"]),
            (197, "Campylobacter jejuni", "species", &["C. jejuni"]),
            (590, "Salmonella enterica", "species", &["Salmonella"]),
            (623, "Shigella flexneri", "species", &["S. flexneri"]),
            (666, "Vibrio cholerae", "species", &["cholera"]),
            (727, "Haemophilus influenzae", "species", &["H. influenzae"]),
            (83333, "Escherichia coli K-12", "no rank", &["E. coli K-12", "K-12"]),

            // === Common Bacteria (Gram-positive) ===
            (1280, "Staphylococcus aureus", "species", &["S. aureus", "staph"]),
            (1282, "Staphylococcus epidermidis", "species", &["S. epidermidis"]),
            (1313, "Streptococcus pneumoniae", "species", &["pneumococcus"]),
            (1314, "Streptococcus pyogenes", "species", &["S. pyogenes", "GAS"]),
            (1311, "Streptococcus agalactiae", "species", &["S. agalactiae", "GBS"]),
            (1423, "Bacillus subtilis", "species", &["B. subtilis"]),
            (1428, "Bacillus thuringiensis", "species", &["Bt"]),
            (1392, "Bacillus anthracis", "species", &["anthrax"]),
            (1496, "Clostridioides difficile", "species", &["C. difficile", "C. diff"]),
            (1502, "Clostridium perfringens", "species", &["C. perfringens"]),
            (1386, "Bacillus", "genus", &[]),
            (1350, "Enterococcus faecalis", "species", &["E. faecalis"]),
            (1351, "Enterococcus faecium", "species", &["E. faecium"]),
            (1773, "Mycobacterium tuberculosis", "species", &["TB", "tuberculosis"]),
            (1769, "Mycobacterium leprae", "species", &["leprosy"]),
            (1763, "Mycobacterium", "genus", &[]),

            // === Gut Microbiome (Common) ===
            (817, "Bacteroides fragilis", "species", &["B. fragilis"]),
            (821, "Bacteroides vulgatus", "species", &["B. vulgatus"]),
            (818, "Bacteroides thetaiotaomicron", "species", &["B. theta"]),
            (816, "Bacteroides", "genus", &[]),
            (853, "Faecalibacterium prausnitzii", "species", &["F. prausnitzii"]),
            (39491, "Eubacterium rectale", "species", &["E. rectale"]),
            (33039, "Ruminococcus bromii", "species", &["R. bromii"]),
            (1263, "Ruminococcus", "genus", &[]),
            (216816, "Bifidobacterium longum", "species", &["B. longum"]),
            (1678, "Bifidobacterium", "genus", &[]),
            (1680, "Bifidobacterium adolescentis", "species", &["B. adolescentis"]),
            (28116, "Bacteroides ovatus", "species", &["B. ovatus"]),
            (28118, "Bacteroides uniformis", "species", &["B. uniformis"]),
            (29466, "Prevotella copri", "species", &["P. copri"]),
            (838, "Prevotella", "genus", &[]),
            (1578, "Lactobacillus", "genus", &[]),
            (1598, "Lactobacillus reuteri", "species", &["L. reuteri"]),
            (1613, "Lactobacillus fermentum", "species", &["L. fermentum"]),
            (47715, "Lactobacillus rhamnosus", "species", &["L. rhamnosus", "LGG"]),
            (1590, "Lactobacillus plantarum", "species", &["L. plantarum"]),
            (1610, "Lactobacillus delbrueckii", "species", &["L. delbrueckii"]),
            (33958, "Lactobacillus acidophilus", "species", &["L. acidophilus"]),
            (186826, "Lactobacillus casei", "species", &["L. casei"]),
            (29375, "Akkermansia muciniphila", "species", &["A. muciniphila"]),
            (239935, "Akkermansia", "genus", &[]),
            (46503, "Dialister invisus", "species", &["D. invisus"]),
            (39778, "Enterobacteriaceae", "family", &[]),

            // === Oral Microbiome ===
            (1303, "Streptococcus oralis", "species", &["S. oralis"]),
            (1304, "Streptococcus salivarius", "species", &["S. salivarius"]),
            (1338, "Streptococcus sanguinis", "species", &["S. sanguinis"]),
            (1305, "Streptococcus mutans", "species", &["S. mutans"]),
            (837, "Porphyromonas gingivalis", "species", &["P. gingivalis"]),
            (851, "Fusobacterium nucleatum", "species", &["F. nucleatum"]),
            (28132, "Prevotella intermedia", "species", &["P. intermedia"]),
            (712, "Pasteurella multocida", "species", &["P. multocida"]),

            // === Skin Microbiome ===
            (1747, "Cutibacterium acnes", "species", &["P. acnes", "Propionibacterium acnes"]),
            (29388, "Corynebacterium", "genus", &[]),
            (1718, "Corynebacterium diphtheriae", "species", &["diphtheria"]),
            (38289, "Malassezia", "genus", &[]),
            (76773, "Malassezia globosa", "species", &["M. globosa"]),

            // === Environmental Bacteria ===
            (303, "Pseudomonas", "genus", &[]),
            (294, "Pseudomonas fluorescens", "species", &["P. fluorescens"]),
            (286, "Pseudomonas syringae", "species", &["P. syringae"]),
            (1279, "Staphylococcus", "genus", &[]),
            (1301, "Streptococcus", "genus", &[]),
            (1760, "Actinobacteria", "phylum", &[]),
            (1236, "Gammaproteobacteria", "class", &[]),
            (1224, "Proteobacteria", "phylum", &[]),
            (1239, "Firmicutes", "phylum", &[]),
            (976, "Bacteroidetes", "phylum", &[]),

            // === Pathogens ===
            (632, "Yersinia pestis", "species", &["plague"]),
            (773, "Listeria monocytogenes", "species", &["L. monocytogenes", "listeria"]),
            (777, "Legionella pneumophila", "species", &["Legionnaire's"]),
            (446, "Legionella", "genus", &[]),
            (90371, "Salmonella enterica subsp. enterica", "subspecies", &[]),
            (813, "Chlamydia trachomatis", "species", &["chlamydia"]),
            (782, "Rickettsia", "genus", &[]),
            (780, "Rickettsia prowazekii", "species", &["typhus"]),
            (833, "Treponema pallidum", "species", &["syphilis"]),
            (293, "Bradyrhizobium japonicum", "species", &["B. japonicum"]),

            // === Archaea ===
            (2157, "Archaea", "superkingdom", &[]),
            (2287, "Methanobacterium", "genus", &[]),
            (2190, "Methanosarcina", "genus", &[]),
            (183968, "Methanobrevibacter smithii", "species", &["M. smithii"]),

            // === Fungi ===
            (4751, "Fungi", "kingdom", &["fungi"]),
            (5476, "Candida albicans", "species", &["C. albicans", "candida"]),
            (5478, "Candida tropicalis", "species", &["C. tropicalis"]),
            (5482, "Candida glabrata", "species", &["C. glabrata"]),
            (5475, "Candida", "genus", &[]),
            (5062, "Aspergillus fumigatus", "species", &["A. fumigatus"]),
            (5061, "Aspergillus niger", "species", &["A. niger"]),
            (5052, "Aspergillus", "genus", &[]),
            (5141, "Neurospora crassa", "species", &["N. crassa", "red bread mold"]),
            (746128, "Aspergillus fumigatus Af293", "no rank", &[]),

            // === Viruses (Common) ===
            (10239, "Viruses", "superkingdom", &["virus"]),
            (11103, "Hepatitis C virus", "species", &["HCV"]),
            (10298, "Human alphaherpesvirus 1", "species", &["HSV-1", "herpes simplex 1"]),
            (10310, "Human alphaherpesvirus 2", "species", &["HSV-2", "herpes simplex 2"]),
            (10359, "Human gammaherpesvirus 4", "species", &["EBV", "Epstein-Barr"]),
            (10376, "Human betaherpesvirus 5", "species", &["CMV", "cytomegalovirus"]),
            (11676, "Human immunodeficiency virus 1", "species", &["HIV-1", "HIV"]),
            (211044, "Influenza A virus", "species", &["influenza A", "flu A"]),
            (11320, "Influenza A virus (A/Puerto Rico/8/1934(H1N1))", "no rank", &["PR8"]),
            (2697049, "Severe acute respiratory syndrome coronavirus 2", "species", &["SARS-CoV-2", "COVID-19"]),
            (694009, "Severe acute respiratory syndrome-related coronavirus", "species", &["SARS-CoV"]),
            (12814, "Respiratory syncytial virus", "species", &["RSV"]),
            (12637, "Rotavirus", "genus", &["rotavirus"]),
            (10632, "Tobacco mosaic virus", "species", &["TMV"]),
            (10665, "Bacteriophage T4", "species", &["T4 phage"]),
            (10710, "Lambda phage", "species", &["bacteriophage lambda"]),

            // === Metagenomes ===
            (408170, "human gut metagenome", "no rank", &["gut metagenome"]),
            (412755, "marine metagenome", "no rank", &[]),
            (410658, "soil metagenome", "no rank", &[]),
            (433733, "human skin metagenome", "no rank", &["skin metagenome"]),
            (447426, "human oral metagenome", "no rank", &["oral metagenome"]),
            (749907, "human vaginal metagenome", "no rank", &["vaginal metagenome"]),
            (556182, "freshwater metagenome", "no rank", &[]),
            (652676, "hypersaline lake metagenome", "no rank", &[]),
            (527639, "wastewater metagenome", "no rank", &[]),
            (410657, "air metagenome", "no rank", &[]),
            (428143, "human lung metagenome", "no rank", &["lung metagenome"]),
            (870726, "human milk metagenome", "no rank", &["milk metagenome"]),

            // === Plants (Common) ===
            (4577, "Zea mays", "species", &["corn", "maize"]),
            (4530, "Oryza sativa", "species", &["rice"]),
            (4565, "Triticum aestivum", "species", &["wheat", "bread wheat"]),
            (3847, "Glycine max", "species", &["soybean"]),
            (3750, "Malus domestica", "species", &["apple"]),
            (4081, "Solanum lycopersicum", "species", &["tomato"]),
            (4113, "Solanum tuberosum", "species", &["potato"]),
            (3760, "Prunus persica", "species", &["peach"]),
        ];

        for (taxid, name, rank, common) in organisms {
            let entry = TaxonomyEntry {
                taxid: *taxid,
                scientific_name: name.to_string(),
                rank: rank.to_string(),
                common_names: common.iter().map(|s| s.to_string()).collect(),
                parent_taxid: None, // Could add lineage later
            };

            // Index by lowercase scientific name
            self.known_organisms
                .insert(name.to_lowercase(), entry.clone());

            // Also index by common names
            for common_name in &entry.common_names {
                self.known_organisms
                    .insert(common_name.to_lowercase(), entry.clone());
            }

            // Index by taxid
            self.taxid_index.insert(*taxid, entry);
        }
    }

    /// Check if a value is a known abbreviation.
    pub fn expand_abbreviation(&self, value: &str) -> Option<&str> {
        self.abbreviations.get(&value.to_lowercase()).map(|s| s.as_str())
    }

    /// Look up an organism by name.
    pub fn lookup(&self, name: &str) -> Option<&TaxonomyEntry> {
        let name_lower = name.to_lowercase().trim().to_string();

        // Direct lookup
        if let Some(entry) = self.known_organisms.get(&name_lower) {
            return Some(entry);
        }

        // Try expanding abbreviation
        if let Some(expanded) = self.expand_abbreviation(&name_lower) {
            return self.known_organisms.get(&expanded.to_lowercase());
        }

        None
    }

    /// Validate an organism name and return suggestions.
    pub fn validate(&self, name: &str) -> TaxonomyValidationResult {
        let name_trimmed = name.trim();

        // Empty or placeholder values
        if name_trimmed.is_empty()
            || name_trimmed.to_lowercase() == "na"
            || name_trimmed.to_lowercase() == "unknown"
        {
            return TaxonomyValidationResult::Invalid {
                reason: "Empty or placeholder taxonomy value".to_string(),
            };
        }

        // Check if it's a known abbreviation
        if let Some(expanded) = self.expand_abbreviation(name_trimmed) {
            if let Some(entry) = self.known_organisms.get(&expanded.to_lowercase()) {
                return TaxonomyValidationResult::Abbreviation {
                    input: name_trimmed.to_string(),
                    expanded: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                };
            }
        }

        // Direct lookup
        if let Some(entry) = self.lookup(name_trimmed) {
            // Check if case is correct
            if entry.scientific_name != name_trimmed {
                return TaxonomyValidationResult::CaseError {
                    input: name_trimmed.to_string(),
                    correct: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                };
            }
            return TaxonomyValidationResult::Valid {
                scientific_name: entry.scientific_name.clone(),
                taxid: entry.taxid,
            };
        }

        // Try fuzzy matching (simple Levenshtein for now)
        if let Some((entry, distance)) = self.fuzzy_match(name_trimmed) {
            if distance <= 2 {
                return TaxonomyValidationResult::PossibleTypo {
                    input: name_trimmed.to_string(),
                    suggestion: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                    distance,
                };
            }
        }

        // Unknown taxonomy
        TaxonomyValidationResult::Unknown {
            input: name_trimmed.to_string(),
        }
    }

    /// Simple fuzzy matching using Levenshtein distance.
    fn fuzzy_match(&self, name: &str) -> Option<(&TaxonomyEntry, usize)> {
        let name_lower = name.to_lowercase();
        let mut best_match: Option<(&TaxonomyEntry, usize)> = None;

        for entry in self.known_organisms.values() {
            let distance = levenshtein(&name_lower, &entry.scientific_name.to_lowercase());
            if distance <= 3 {
                match &best_match {
                    None => best_match = Some((entry, distance)),
                    Some((_, best_dist)) if distance < *best_dist => {
                        best_match = Some((entry, distance));
                    }
                    _ => {}
                }
            }
        }

        best_match
    }
}

impl Default for TaxonomyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of taxonomy validation.
#[derive(Debug, Clone)]
pub enum TaxonomyValidationResult {
    /// The name is valid and matches NCBI Taxonomy.
    Valid { scientific_name: String, taxid: u32 },
    /// The name is an abbreviation that should be expanded.
    Abbreviation {
        input: String,
        expanded: String,
        taxid: u32,
    },
    /// The name has incorrect capitalization.
    CaseError {
        input: String,
        correct: String,
        taxid: u32,
    },
    /// The name appears to be a typo.
    PossibleTypo {
        input: String,
        suggestion: String,
        taxid: u32,
        distance: usize,
    },
    /// The name is not recognized in our database.
    Unknown { input: String },
    /// The value is invalid (empty, placeholder, etc.).
    Invalid { reason: String },
}

/// Simple Levenshtein distance implementation.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_abbreviation() {
        let validator = TaxonomyValidator::new();
        assert_eq!(
            validator.expand_abbreviation("E. coli"),
            Some("Escherichia coli")
        );
        assert_eq!(
            validator.expand_abbreviation("e coli"),
            Some("Escherichia coli")
        );
    }

    #[test]
    fn test_lookup_organism() {
        let validator = TaxonomyValidator::new();

        let result = validator.lookup("Escherichia coli");
        assert!(result.is_some());
        assert_eq!(result.unwrap().taxid, 562);

        let result = validator.lookup("human");
        assert!(result.is_some());
        assert_eq!(result.unwrap().taxid, 9606);
    }

    #[test]
    fn test_validate_valid() {
        let validator = TaxonomyValidator::new();

        match validator.validate("Escherichia coli") {
            TaxonomyValidationResult::Valid { taxid, .. } => assert_eq!(taxid, 562),
            other => panic!("Expected Valid, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_abbreviation() {
        let validator = TaxonomyValidator::new();

        match validator.validate("E. coli") {
            TaxonomyValidationResult::Abbreviation { taxid, expanded, .. } => {
                assert_eq!(taxid, 562);
                assert_eq!(expanded, "Escherichia coli");
            }
            other => panic!("Expected Abbreviation, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_case_error() {
        let validator = TaxonomyValidator::new();

        match validator.validate("escherichia coli") {
            TaxonomyValidationResult::CaseError { correct, .. } => {
                assert_eq!(correct, "Escherichia coli");
            }
            other => panic!("Expected CaseError, got {:?}", other),
        }
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("hello", "hello"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
    }
}
