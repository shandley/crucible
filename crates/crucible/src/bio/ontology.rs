//! Biological ontology support for metadata validation.
//!
//! This module provides validation and mapping of free-text terms to standard
//! biological ontologies including:
//!
//! - ENVO (Environmental Ontology) - for environmental terms
//! - UBERON (Uber-anatomy Ontology) - for anatomical terms
//! - MONDO (Monarch Disease Ontology) - for disease terms
//!
//! # Example
//!
//! ```ignore
//! use crucible::bio::OntologyValidator;
//!
//! let validator = OntologyValidator::new();
//!
//! // Look up a term
//! if let Some(term) = validator.lookup("gut") {
//!     println!("{}: {} ({})", term.id, term.label, term.ontology);
//! }
//!
//! // Suggest mappings for free text
//! let suggestions = validator.suggest_mappings("human feces");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Supported ontology types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OntologyType {
    /// Environmental Ontology (ENVO)
    Envo,
    /// Uber-anatomy Ontology (UBERON)
    Uberon,
    /// Monarch Disease Ontology (MONDO)
    Mondo,
    /// Cell Ontology (CL)
    CellOntology,
    /// Gene Ontology (GO)
    GeneOntology,
}

impl OntologyType {
    /// Get the ontology prefix used in IDs.
    pub fn prefix(&self) -> &'static str {
        match self {
            OntologyType::Envo => "ENVO",
            OntologyType::Uberon => "UBERON",
            OntologyType::Mondo => "MONDO",
            OntologyType::CellOntology => "CL",
            OntologyType::GeneOntology => "GO",
        }
    }

    /// Get the human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            OntologyType::Envo => "Environmental Ontology",
            OntologyType::Uberon => "Uber-anatomy Ontology",
            OntologyType::Mondo => "Monarch Disease Ontology",
            OntologyType::CellOntology => "Cell Ontology",
            OntologyType::GeneOntology => "Gene Ontology",
        }
    }

    /// Parse ontology type from an ID prefix.
    pub fn from_id(id: &str) -> Option<Self> {
        let id_upper = id.to_uppercase();
        if id_upper.starts_with("ENVO:") {
            Some(OntologyType::Envo)
        } else if id_upper.starts_with("UBERON:") {
            Some(OntologyType::Uberon)
        } else if id_upper.starts_with("MONDO:") {
            Some(OntologyType::Mondo)
        } else if id_upper.starts_with("CL:") {
            Some(OntologyType::CellOntology)
        } else if id_upper.starts_with("GO:") {
            Some(OntologyType::GeneOntology)
        } else {
            None
        }
    }
}

/// An ontology term entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyTerm {
    /// The ontology ID (e.g., "ENVO:00002261").
    pub id: String,
    /// The primary label/name.
    pub label: String,
    /// The ontology this term belongs to.
    pub ontology: OntologyType,
    /// Alternative names/synonyms.
    pub synonyms: Vec<String>,
    /// Brief definition (if available).
    pub definition: Option<String>,
    /// Parent term IDs (for hierarchy lookups).
    pub parent_ids: Vec<String>,
}

impl OntologyTerm {
    /// Create a new ontology term.
    pub fn new(id: impl Into<String>, label: impl Into<String>, ontology: OntologyType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            ontology,
            synonyms: Vec::new(),
            definition: None,
            parent_ids: Vec::new(),
        }
    }

    /// Add a synonym.
    pub fn with_synonym(mut self, synonym: impl Into<String>) -> Self {
        self.synonyms.push(synonym.into());
        self
    }

    /// Add synonyms.
    pub fn with_synonyms(mut self, synonyms: Vec<String>) -> Self {
        self.synonyms = synonyms;
        self
    }

    /// Set the definition.
    pub fn with_definition(mut self, definition: impl Into<String>) -> Self {
        self.definition = Some(definition.into());
        self
    }
}

/// Statistics about loaded ontology data.
#[derive(Debug, Clone, Default)]
pub struct OntologyStats {
    /// Total number of terms loaded.
    pub total_terms: usize,
    /// Terms by ontology type.
    pub terms_by_ontology: HashMap<OntologyType, usize>,
    /// Number of synonyms indexed.
    pub synonym_count: usize,
    /// Data source description.
    pub source: String,
}

/// Ontology validator for mapping and validating ontology terms.
#[derive(Debug, Clone)]
pub struct OntologyValidator {
    /// Terms indexed by ID.
    terms_by_id: HashMap<String, OntologyTerm>,
    /// Terms indexed by lowercase label.
    terms_by_label: HashMap<String, Vec<OntologyTerm>>,
    /// Terms indexed by lowercase synonym.
    terms_by_synonym: HashMap<String, Vec<OntologyTerm>>,
    /// Statistics about loaded data.
    stats: OntologyStats,
}

impl OntologyValidator {
    /// Create a new ontology validator with built-in common terms.
    pub fn new() -> Self {
        let mut validator = Self {
            terms_by_id: HashMap::new(),
            terms_by_label: HashMap::new(),
            terms_by_synonym: HashMap::new(),
            stats: OntologyStats {
                source: "built-in".to_string(),
                ..Default::default()
            },
        };
        validator.load_common_envo_terms();
        validator.load_common_uberon_terms();
        validator.load_common_mondo_terms();
        validator.update_stats();
        validator
    }

    /// Load terms from an OBO format file.
    pub fn load_obo_file(&mut self, path: impl AsRef<Path>) -> Result<usize, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut current_term: Option<OntologyTerm> = None;
        let mut terms_loaded = 0;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line == "[Term]" {
                // Save previous term if exists
                if let Some(term) = current_term.take() {
                    self.add_term(term);
                    terms_loaded += 1;
                }
                current_term = None;
            } else if line.starts_with("id: ") {
                let id = line[4..].trim().to_string();
                if let Some(ontology) = OntologyType::from_id(&id) {
                    current_term = Some(OntologyTerm::new(&id, "", ontology));
                }
            } else if let Some(ref mut term) = current_term {
                if line.starts_with("name: ") {
                    term.label = line[6..].trim().to_string();
                } else if line.starts_with("def: ") {
                    // Parse definition: "definition text" [source]
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            term.definition = Some(line[start + 1..start + 1 + end].to_string());
                        }
                    }
                } else if line.starts_with("synonym: ") {
                    // Parse synonym: "synonym text" TYPE [source]
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            term.synonyms.push(line[start + 1..start + 1 + end].to_string());
                        }
                    }
                } else if line.starts_with("is_a: ") {
                    let parent = line[6..].split_whitespace().next().unwrap_or("").to_string();
                    if !parent.is_empty() {
                        term.parent_ids.push(parent);
                    }
                }
            }
        }

        // Save last term
        if let Some(term) = current_term {
            self.add_term(term);
            terms_loaded += 1;
        }

        self.update_stats();
        Ok(terms_loaded)
    }

    /// Add a term to the validator.
    pub fn add_term(&mut self, term: OntologyTerm) {
        // Index by ID
        let id_upper = term.id.to_uppercase();
        self.terms_by_id.insert(id_upper, term.clone());

        // Index by label
        let label_lower = term.label.to_lowercase();
        self.terms_by_label
            .entry(label_lower)
            .or_default()
            .push(term.clone());

        // Index by synonyms
        for synonym in &term.synonyms {
            let syn_lower = synonym.to_lowercase();
            self.terms_by_synonym
                .entry(syn_lower)
                .or_default()
                .push(term.clone());
        }
    }

    /// Update statistics.
    fn update_stats(&mut self) {
        self.stats.total_terms = self.terms_by_id.len();
        self.stats.synonym_count = self.terms_by_synonym.len();

        self.stats.terms_by_ontology.clear();
        for term in self.terms_by_id.values() {
            *self.stats.terms_by_ontology.entry(term.ontology).or_insert(0) += 1;
        }
    }

    /// Get statistics about loaded ontology data.
    pub fn stats(&self) -> &OntologyStats {
        &self.stats
    }

    /// Look up a term by ID.
    pub fn lookup_by_id(&self, id: &str) -> Option<&OntologyTerm> {
        self.terms_by_id.get(&id.to_uppercase())
    }

    /// Look up terms by label (exact match).
    pub fn lookup_by_label(&self, label: &str) -> Vec<&OntologyTerm> {
        self.terms_by_label
            .get(&label.to_lowercase())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Look up terms by label or synonym.
    pub fn lookup(&self, text: &str) -> Vec<&OntologyTerm> {
        let text_lower = text.to_lowercase();

        // First try exact label match
        if let Some(terms) = self.terms_by_label.get(&text_lower) {
            return terms.iter().collect();
        }

        // Then try synonym match
        if let Some(terms) = self.terms_by_synonym.get(&text_lower) {
            return terms.iter().collect();
        }

        Vec::new()
    }

    /// Validate an ontology ID.
    pub fn validate_id(&self, id: &str) -> OntologyValidationResult {
        let id_upper = id.to_uppercase();

        // Check if ID format is valid
        if let Some(ontology) = OntologyType::from_id(&id_upper) {
            // Check if term exists in our database
            if let Some(term) = self.terms_by_id.get(&id_upper) {
                return OntologyValidationResult::Valid {
                    id: term.id.clone(),
                    label: term.label.clone(),
                    ontology,
                };
            }

            // Valid format but unknown term
            return OntologyValidationResult::UnknownTerm {
                id: id.to_string(),
                ontology,
            };
        }

        // Invalid ID format
        OntologyValidationResult::InvalidFormat {
            input: id.to_string(),
        }
    }

    /// Suggest ontology mappings for free text.
    pub fn suggest_mappings(&self, text: &str, ontology_filter: Option<OntologyType>) -> Vec<OntologyMapping> {
        let text_lower = text.to_lowercase().trim().to_string();
        let mut mappings = Vec::new();

        // Direct label match
        if let Some(terms) = self.terms_by_label.get(&text_lower) {
            for term in terms {
                if ontology_filter.is_none() || ontology_filter == Some(term.ontology) {
                    mappings.push(OntologyMapping {
                        input: text.to_string(),
                        term_id: term.id.clone(),
                        term_label: term.label.clone(),
                        ontology: term.ontology,
                        match_type: MatchType::ExactLabel,
                        confidence: 1.0,
                    });
                }
            }
        }

        // Synonym match
        if let Some(terms) = self.terms_by_synonym.get(&text_lower) {
            for term in terms {
                if ontology_filter.is_none() || ontology_filter == Some(term.ontology) {
                    // Find which synonym matched
                    let matched_syn = term
                        .synonyms
                        .iter()
                        .find(|s| s.to_lowercase() == text_lower)
                        .cloned()
                        .unwrap_or_default();

                    mappings.push(OntologyMapping {
                        input: text.to_string(),
                        term_id: term.id.clone(),
                        term_label: term.label.clone(),
                        ontology: term.ontology,
                        match_type: MatchType::Synonym(matched_syn),
                        confidence: 0.95,
                    });
                }
            }
        }

        // Partial/fuzzy matching for common patterns
        if mappings.is_empty() {
            // Try word-based matching
            let words: Vec<&str> = text_lower.split_whitespace().collect();

            for (label, terms) in &self.terms_by_label {
                for term in terms {
                    if ontology_filter.is_some() && ontology_filter != Some(term.ontology) {
                        continue;
                    }

                    // Check if all words appear in the label
                    let label_lower = label.to_lowercase();
                    if words.iter().all(|w| label_lower.contains(w)) {
                        mappings.push(OntologyMapping {
                            input: text.to_string(),
                            term_id: term.id.clone(),
                            term_label: term.label.clone(),
                            ontology: term.ontology,
                            match_type: MatchType::Partial,
                            confidence: 0.7,
                        });
                    }
                }
            }
        }

        // Sort by confidence
        mappings.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // Deduplicate by term ID
        let mut seen = std::collections::HashSet::new();
        mappings.retain(|m| seen.insert(m.term_id.clone()));

        mappings
    }

    /// Load common ENVO terms for environmental metadata.
    fn load_common_envo_terms(&mut self) {
        let terms = [
            // Biomes (env_broad_scale)
            ("ENVO:00000446", "terrestrial biome", &["land biome"][..]),
            ("ENVO:00000447", "marine biome", &["ocean biome", "sea biome"]),
            ("ENVO:00000873", "freshwater biome", &["freshwater"]),
            ("ENVO:01000174", "forest biome", &["forest"]),
            ("ENVO:01000175", "woodland biome", &["woodland"]),
            ("ENVO:01000176", "shrubland biome", &["shrubland"]),
            ("ENVO:01000177", "grassland biome", &["grassland", "prairie"]),
            ("ENVO:01000178", "savanna biome", &["savanna"]),
            ("ENVO:01000179", "desert biome", &["desert"]),
            ("ENVO:01000180", "tundra biome", &["tundra"]),
            ("ENVO:01000181", "mangrove biome", &["mangrove"]),
            ("ENVO:01000219", "anthropogenic terrestrial biome", &["urban biome", "agricultural biome"]),
            ("ENVO:01000246", "village biome", &["village"]),
            ("ENVO:01000248", "dense settlement biome", &["city", "urban area"]),

            // Environmental features (env_local_scale)
            ("ENVO:00000067", "cave", &["cave system"]),
            ("ENVO:00000073", "building", &["structure"]),
            ("ENVO:00000076", "mine", &["mining site"]),
            ("ENVO:00000114", "agricultural field", &["farm field", "cropland"]),
            ("ENVO:00000233", "beach", &["shoreline"]),
            ("ENVO:00000384", "river bed", &["riverbed"]),
            ("ENVO:00002006", "lake shore", &["lakeshore"]),
            ("ENVO:00002007", "sediment", &["sediment layer"]),
            ("ENVO:00002030", "aquatic environment", &["aquatic"]),
            ("ENVO:00002031", "wastewater", &["sewage"]),
            ("ENVO:00002034", "biofilm", &["microbial mat"]),
            ("ENVO:00002149", "salt lake", &["saline lake"]),
            ("ENVO:00002150", "coastal water", &["coastal"]),
            ("ENVO:00002151", "sea water", &["seawater", "ocean water"]),
            ("ENVO:00002228", "hospital environment", &["hospital", "clinical environment"]),
            ("ENVO:00003074", "constructed habitat", &["built environment"]),

            // Environmental materials (env_medium)
            ("ENVO:00001998", "soil", &["dirt", "earth"]),
            ("ENVO:00002001", "water", &["aqueous"]),
            ("ENVO:00002003", "feces", &["fecal matter", "stool"]),
            ("ENVO:00002005", "air", &["atmosphere"]),
            ("ENVO:00002010", "sediment", &[]),
            ("ENVO:00002011", "sludge", &["biosolids"]),
            ("ENVO:00002012", "compost", &["composted material"]),
            ("ENVO:00002016", "hypersaline water", &["brine"]),
            ("ENVO:00002042", "surface water", &[]),
            ("ENVO:00002044", "groundwater", &["ground water"]),
            ("ENVO:00002045", "drinking water", &["potable water"]),
            ("ENVO:00002148", "wetland", &["marsh", "swamp"]),
            ("ENVO:00002261", "forest soil", &["woodland soil"]),
            ("ENVO:00002262", "agricultural soil", &["farm soil", "crop soil"]),
            ("ENVO:00005789", "human gut", &["intestinal content", "gut content"]),
            ("ENVO:00005792", "saliva", &["oral fluid"]),
            ("ENVO:00010505", "mucus", &["mucosal secretion"]),
            ("ENVO:01000155", "organic material", &["organic matter"]),
            ("ENVO:01000306", "urine", &[]),
            ("ENVO:01000321", "blood", &[]),
            ("ENVO:02000033", "rhizosphere", &["root zone"]),
            ("ENVO:00002259", "sand", &[]),
            ("ENVO:00002018", "clay", &[]),
        ];

        for (id, label, synonyms) in terms {
            let term = OntologyTerm::new(id, label, OntologyType::Envo)
                .with_synonyms(synonyms.iter().map(|s| s.to_string()).collect());
            self.add_term(term);
        }
    }

    /// Load common UBERON terms for anatomical metadata.
    fn load_common_uberon_terms(&mut self) {
        let terms = [
            // Body sites
            ("UBERON:0000160", "intestine", &["gut", "bowel"][..]),
            ("UBERON:0000945", "stomach", &["gastric"]),
            ("UBERON:0000955", "brain", &["cerebral"]),
            ("UBERON:0000970", "eye", &["ocular"]),
            ("UBERON:0000990", "reproductive system", &["genital"]),
            ("UBERON:0001004", "respiratory system", &["respiratory tract"]),
            ("UBERON:0001007", "digestive system", &["gastrointestinal tract", "GI tract"]),
            ("UBERON:0001009", "circulatory system", &["cardiovascular system"]),
            ("UBERON:0001013", "adipose tissue", &["fat tissue"]),
            ("UBERON:0001016", "nervous system", &["neural"]),
            ("UBERON:0001043", "esophagus", &["oesophagus"]),
            ("UBERON:0001052", "rectum", &["rectal"]),
            ("UBERON:0001088", "urine", &["urinary"]),
            ("UBERON:0001134", "skeletal muscle tissue", &["muscle"]),
            ("UBERON:0001155", "colon", &["large intestine"]),
            ("UBERON:0001264", "pancreas", &["pancreatic"]),
            ("UBERON:0001474", "bone element", &["bone"]),
            ("UBERON:0001836", "saliva", &["oral fluid"]),
            ("UBERON:0001911", "mammary gland", &["breast"]),
            ("UBERON:0001988", "feces", &["stool", "fecal matter"]),
            ("UBERON:0002048", "lung", &["pulmonary"]),
            ("UBERON:0002097", "skin of body", &["skin", "dermal"]),
            ("UBERON:0002106", "spleen", &["splenic"]),
            ("UBERON:0002107", "liver", &["hepatic"]),
            ("UBERON:0002110", "gall bladder", &["gallbladder"]),
            ("UBERON:0002113", "kidney", &["renal"]),
            ("UBERON:0002114", "duodenum", &["duodenal"]),
            ("UBERON:0002115", "jejunum", &["jejunal"]),
            ("UBERON:0002116", "ileum", &["ileal"]),
            ("UBERON:0002367", "prostate gland", &["prostate"]),
            ("UBERON:0002370", "thymus", &["thymic"]),
            ("UBERON:0002371", "bone marrow", &["marrow"]),
            ("UBERON:0003126", "trachea", &["windpipe"]),
            ("UBERON:0006314", "body fluid", &["biological fluid"]),
            ("UBERON:0000178", "blood", &["peripheral blood"]),
            ("UBERON:0001456", "face", &["facial"]),
            ("UBERON:0001637", "artery", &["arterial"]),
            ("UBERON:0001913", "milk", &["breast milk"]),
            ("UBERON:0000996", "vagina", &["vaginal"]),
            ("UBERON:0000995", "uterus", &["uterine"]),
            ("UBERON:0001245", "anus", &["anal"]),
            ("UBERON:0001707", "nasal cavity", &["nose", "nasal"]),
            ("UBERON:0001723", "tongue", &["lingual"]),
            ("UBERON:0001729", "oropharynx", &["throat"]),
            ("UBERON:0001982", "hair", &["scalp hair"]),
            ("UBERON:0003104", "urethra", &["urethral"]),
            ("UBERON:0018707", "bladder organ", &["urinary bladder"]),
            ("UBERON:0000341", "throat", &["pharynx"]),
            ("UBERON:0000948", "heart", &["cardiac"]),
        ];

        for (id, label, synonyms) in terms {
            let term = OntologyTerm::new(id, label, OntologyType::Uberon)
                .with_synonyms(synonyms.iter().map(|s| s.to_string()).collect());
            self.add_term(term);
        }
    }

    /// Load common MONDO terms for disease metadata.
    fn load_common_mondo_terms(&mut self) {
        let terms = [
            // Gastrointestinal diseases
            ("MONDO:0005011", "Crohn disease", &["Crohn's disease", "CD", "regional enteritis"][..]),
            ("MONDO:0005265", "inflammatory bowel disease", &["IBD"]),
            ("MONDO:0005101", "ulcerative colitis", &["UC"]),
            ("MONDO:0005052", "irritable bowel syndrome", &["IBS"]),
            ("MONDO:0000861", "colorectal cancer", &["colon cancer", "CRC"]),
            ("MONDO:0005170", "gastric cancer", &["stomach cancer"]),
            ("MONDO:0001974", "gastritis", &[]),
            ("MONDO:0004992", "celiac disease", &["coeliac disease", "celiac sprue"]),

            // Metabolic diseases
            ("MONDO:0005015", "diabetes mellitus", &["diabetes"]),
            ("MONDO:0005148", "type 2 diabetes mellitus", &["T2D", "type 2 diabetes"]),
            ("MONDO:0005147", "type 1 diabetes mellitus", &["T1D", "type 1 diabetes"]),
            ("MONDO:0011122", "obesity", &[]),
            ("MONDO:0004974", "liver disease", &["hepatic disease"]),
            ("MONDO:0005359", "non-alcoholic fatty liver disease", &["NAFLD"]),

            // Cardiovascular diseases
            ("MONDO:0005267", "heart disease", &["cardiac disease"]),
            ("MONDO:0005068", "myocardial infarction", &["heart attack", "MI"]),
            ("MONDO:0001134", "coronary artery disease", &["CAD"]),
            ("MONDO:0005044", "hypertension", &["high blood pressure"]),
            ("MONDO:0005091", "atherosclerosis", &[]),

            // Neurological diseases
            ("MONDO:0004975", "Alzheimer disease", &["Alzheimer's disease", "AD"]),
            ("MONDO:0007739", "Huntington disease", &["Huntington's disease", "HD"]),
            ("MONDO:0005180", "Parkinson disease", &["Parkinson's disease", "PD"]),
            ("MONDO:0004985", "multiple sclerosis", &["MS"]),
            ("MONDO:0001071", "depression", &["major depressive disorder", "MDD"]),
            ("MONDO:0008315", "autism spectrum disorder", &["autism", "ASD"]),

            // Autoimmune diseases
            ("MONDO:0005623", "autoimmune disease", &["autoimmunity"]),
            ("MONDO:0005294", "rheumatoid arthritis", &["RA"]),
            ("MONDO:0001014", "lupus", &["systemic lupus erythematosus", "SLE"]),
            ("MONDO:0005520", "multiple sclerosis", &["MS"]),
            ("MONDO:0011649", "psoriasis", &[]),

            // Infectious diseases
            ("MONDO:0100096", "COVID-19", &["coronavirus disease 2019", "SARS-CoV-2 infection"]),
            ("MONDO:0005109", "HIV infection", &["HIV/AIDS", "AIDS"]),
            ("MONDO:0018076", "tuberculosis", &["TB"]),
            ("MONDO:0005812", "influenza", &["flu"]),
            ("MONDO:0004609", "hepatitis B", &["HBV infection"]),
            ("MONDO:0004610", "hepatitis C", &["HCV infection"]),
            ("MONDO:0005737", "Ebola hemorrhagic fever", &["Ebola"]),

            // Cancer types
            ("MONDO:0004992", "cancer", &["malignancy", "malignant neoplasm"]),
            ("MONDO:0008903", "lung cancer", &["lung carcinoma"]),
            ("MONDO:0006256", "breast cancer", &["breast carcinoma"]),
            ("MONDO:0008315", "prostate cancer", &["prostate carcinoma"]),
            ("MONDO:0001056", "leukemia", &[]),
            ("MONDO:0024880", "melanoma", &[]),

            // Respiratory diseases
            ("MONDO:0004979", "asthma", &[]),
            ("MONDO:0005002", "chronic obstructive pulmonary disease", &["COPD"]),
            ("MONDO:0002254", "pneumonia", &[]),
            ("MONDO:0001490", "cystic fibrosis", &["CF"]),

            // Other common conditions
            ("MONDO:0005027", "allergy", &["allergic disease"]),
            ("MONDO:0005364", "atopic dermatitis", &["eczema"]),
            ("MONDO:0021113", "osteoarthritis", &["OA"]),
            ("MONDO:0005147", "osteoporosis", &[]),
            ("MONDO:0002462", "periodontitis", &["periodontal disease", "gum disease"]),
        ];

        for (id, label, synonyms) in terms {
            let term = OntologyTerm::new(id, label, OntologyType::Mondo)
                .with_synonyms(synonyms.iter().map(|s| s.to_string()).collect());
            self.add_term(term);
        }
    }
}

impl Default for OntologyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of validating an ontology ID.
#[derive(Debug, Clone)]
pub enum OntologyValidationResult {
    /// The ID is valid and the term exists.
    Valid {
        id: String,
        label: String,
        ontology: OntologyType,
    },
    /// The ID format is valid but the term is not in our database.
    UnknownTerm {
        id: String,
        ontology: OntologyType,
    },
    /// The ID format is invalid.
    InvalidFormat {
        input: String,
    },
}

/// A suggested ontology mapping.
#[derive(Debug, Clone)]
pub struct OntologyMapping {
    /// The original input text.
    pub input: String,
    /// The suggested ontology term ID.
    pub term_id: String,
    /// The term label.
    pub term_label: String,
    /// The ontology type.
    pub ontology: OntologyType,
    /// How the match was made.
    pub match_type: MatchType,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
}

/// How an ontology mapping was matched.
#[derive(Debug, Clone)]
pub enum MatchType {
    /// Exact label match.
    ExactLabel,
    /// Matched via synonym.
    Synonym(String),
    /// Partial/word-based match.
    Partial,
    /// Fuzzy match.
    Fuzzy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontology_type_from_id() {
        assert_eq!(OntologyType::from_id("ENVO:00000446"), Some(OntologyType::Envo));
        assert_eq!(OntologyType::from_id("UBERON:0000160"), Some(OntologyType::Uberon));
        assert_eq!(OntologyType::from_id("MONDO:0005011"), Some(OntologyType::Mondo));
        assert_eq!(OntologyType::from_id("invalid"), None);
    }

    #[test]
    fn test_lookup_by_id() {
        let validator = OntologyValidator::new();

        let term = validator.lookup_by_id("ENVO:00001998");
        assert!(term.is_some());
        assert_eq!(term.unwrap().label, "soil");
    }

    #[test]
    fn test_lookup_by_label() {
        let validator = OntologyValidator::new();

        let terms = validator.lookup_by_label("soil");
        assert!(!terms.is_empty());
        assert!(terms.iter().any(|t| t.id == "ENVO:00001998"));
    }

    #[test]
    fn test_lookup_by_synonym() {
        let validator = OntologyValidator::new();

        // "gut" is a synonym for intestine
        let terms = validator.lookup("gut");
        assert!(!terms.is_empty());
        assert!(terms.iter().any(|t| t.id == "UBERON:0000160"));

        // "stool" is a synonym for feces
        let terms = validator.lookup("stool");
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_validate_id() {
        let validator = OntologyValidator::new();

        // Valid ID in database
        match validator.validate_id("ENVO:00001998") {
            OntologyValidationResult::Valid { label, .. } => {
                assert_eq!(label, "soil");
            }
            other => panic!("Expected Valid, got {:?}", other),
        }

        // Valid format but not in database
        match validator.validate_id("ENVO:99999999") {
            OntologyValidationResult::UnknownTerm { .. } => {}
            other => panic!("Expected UnknownTerm, got {:?}", other),
        }

        // Invalid format
        match validator.validate_id("invalid_id") {
            OntologyValidationResult::InvalidFormat { .. } => {}
            other => panic!("Expected InvalidFormat, got {:?}", other),
        }
    }

    #[test]
    fn test_suggest_mappings() {
        let validator = OntologyValidator::new();

        // Direct label match
        let mappings = validator.suggest_mappings("soil", None);
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.term_id == "ENVO:00001998"));

        // Synonym match
        let mappings = validator.suggest_mappings("gut", Some(OntologyType::Uberon));
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.term_id == "UBERON:0000160"));

        // Disease term
        let mappings = validator.suggest_mappings("Crohn's disease", Some(OntologyType::Mondo));
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.term_id == "MONDO:0005011"));
    }

    #[test]
    fn test_filter_by_ontology() {
        let validator = OntologyValidator::new();

        // Filter for ENVO only
        let mappings = validator.suggest_mappings("soil", Some(OntologyType::Envo));
        assert!(mappings.iter().all(|m| m.ontology == OntologyType::Envo));
    }

    #[test]
    fn test_stats() {
        let validator = OntologyValidator::new();
        let stats = validator.stats();

        assert!(stats.total_terms > 100, "Expected >100 terms, got {}", stats.total_terms);
        assert!(
            stats.terms_by_ontology.get(&OntologyType::Envo).unwrap_or(&0) > &30,
            "Expected >30 ENVO terms"
        );
        assert!(
            stats.terms_by_ontology.get(&OntologyType::Uberon).unwrap_or(&0) > &30,
            "Expected >30 UBERON terms"
        );
        assert!(
            stats.terms_by_ontology.get(&OntologyType::Mondo).unwrap_or(&0) > &40,
            "Expected >40 MONDO terms, got {}",
            stats.terms_by_ontology.get(&OntologyType::Mondo).unwrap_or(&0)
        );
    }
}
