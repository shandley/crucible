//! MIxS (Minimum Information about any (x) Sequence) schema definitions.
//!
//! This module defines the MIxS standard fields and environmental packages
//! as specified by the Genomic Standards Consortium.
//!
//! Reference: https://genomicsstandardsconsortium.github.io/mixs/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requirement level for a MIxS field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MixsFieldRequirement {
    /// Mandatory - must be provided for MIxS compliance.
    Mandatory,
    /// Conditionally mandatory - required in certain contexts.
    Conditional,
    /// Recommended - not required but encouraged.
    Recommended,
    /// Optional - can be provided for additional detail.
    Optional,
}

impl MixsFieldRequirement {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            MixsFieldRequirement::Mandatory => "Mandatory",
            MixsFieldRequirement::Conditional => "Conditional",
            MixsFieldRequirement::Recommended => "Recommended",
            MixsFieldRequirement::Optional => "Optional",
        }
    }

    /// Short code used in MIxS specs.
    pub fn code(&self) -> &'static str {
        match self {
            MixsFieldRequirement::Mandatory => "M",
            MixsFieldRequirement::Conditional => "C",
            MixsFieldRequirement::Recommended => "X",
            MixsFieldRequirement::Optional => "-",
        }
    }
}

/// MIxS environmental package types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MixsPackage {
    /// Air samples
    Air,
    /// Built environment / indoor samples
    BuiltEnvironment,
    /// Host-associated samples (non-human)
    HostAssociated,
    /// Human-associated samples (general)
    HumanAssociated,
    /// Human gut microbiome
    HumanGut,
    /// Human oral microbiome
    HumanOral,
    /// Human skin microbiome
    HumanSkin,
    /// Human vaginal microbiome
    HumanVaginal,
    /// Microbial mat / biofilm samples
    MicrobialMatBiofilm,
    /// Miscellaneous natural or artificial environment
    MiscellaneousNaturalOrArtificialEnvironment,
    /// Plant-associated samples
    PlantAssociated,
    /// Sediment samples
    Sediment,
    /// Soil samples
    Soil,
    /// Wastewater / sludge samples
    WastewaterSludge,
    /// Water samples
    Water,
}

impl MixsPackage {
    /// Get the package name as used in MIxS.
    pub fn name(&self) -> &'static str {
        match self {
            MixsPackage::Air => "air",
            MixsPackage::BuiltEnvironment => "built environment",
            MixsPackage::HostAssociated => "host-associated",
            MixsPackage::HumanAssociated => "human-associated",
            MixsPackage::HumanGut => "human-gut",
            MixsPackage::HumanOral => "human-oral",
            MixsPackage::HumanSkin => "human-skin",
            MixsPackage::HumanVaginal => "human-vaginal",
            MixsPackage::MicrobialMatBiofilm => "microbial mat/biofilm",
            MixsPackage::MiscellaneousNaturalOrArtificialEnvironment => "miscellaneous",
            MixsPackage::PlantAssociated => "plant-associated",
            MixsPackage::Sediment => "sediment",
            MixsPackage::Soil => "soil",
            MixsPackage::WastewaterSludge => "wastewater/sludge",
            MixsPackage::Water => "water",
        }
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            MixsPackage::Air => "Atmospheric samples",
            MixsPackage::BuiltEnvironment => "Indoor/constructed environment samples",
            MixsPackage::HostAssociated => "Samples from host organisms (non-human)",
            MixsPackage::HumanAssociated => "Human-associated samples (general)",
            MixsPackage::HumanGut => "Human gut/intestinal microbiome samples",
            MixsPackage::HumanOral => "Human oral cavity samples",
            MixsPackage::HumanSkin => "Human skin microbiome samples",
            MixsPackage::HumanVaginal => "Human vaginal microbiome samples",
            MixsPackage::MicrobialMatBiofilm => "Microbial mat and biofilm samples",
            MixsPackage::MiscellaneousNaturalOrArtificialEnvironment => {
                "Miscellaneous environment samples"
            }
            MixsPackage::PlantAssociated => "Plant-associated microbiome samples",
            MixsPackage::Sediment => "Sediment samples",
            MixsPackage::Soil => "Soil samples",
            MixsPackage::WastewaterSludge => "Wastewater and sludge samples",
            MixsPackage::Water => "Aquatic/water samples",
        }
    }

    /// Parse a package from string (case-insensitive, flexible matching).
    pub fn from_str_flexible(s: &str) -> Option<Self> {
        let s = s.to_lowercase().replace(['-', '_', ' '], "");
        match s.as_str() {
            "air" => Some(MixsPackage::Air),
            "builtenvironment" | "built" | "indoor" => Some(MixsPackage::BuiltEnvironment),
            "hostassociated" | "host" => Some(MixsPackage::HostAssociated),
            "humanassociated" | "human" => Some(MixsPackage::HumanAssociated),
            "humangut" | "gut" | "intestinal" | "fecal" | "stool" => Some(MixsPackage::HumanGut),
            "humanoral" | "oral" | "mouth" | "saliva" => Some(MixsPackage::HumanOral),
            "humanskin" | "skin" | "dermal" => Some(MixsPackage::HumanSkin),
            "humanvaginal" | "vaginal" => Some(MixsPackage::HumanVaginal),
            "microbialmat" | "biofilm" | "mat" => Some(MixsPackage::MicrobialMatBiofilm),
            "miscellaneous" | "misc" | "other" => {
                Some(MixsPackage::MiscellaneousNaturalOrArtificialEnvironment)
            }
            "plantassociated" | "plant" => Some(MixsPackage::PlantAssociated),
            "sediment" => Some(MixsPackage::Sediment),
            "soil" => Some(MixsPackage::Soil),
            "wastewater" | "sludge" | "sewage" => Some(MixsPackage::WastewaterSludge),
            "water" | "aquatic" | "marine" | "freshwater" => Some(MixsPackage::Water),
            _ => None,
        }
    }

    /// Get all packages.
    pub fn all() -> &'static [MixsPackage] {
        &[
            MixsPackage::Air,
            MixsPackage::BuiltEnvironment,
            MixsPackage::HostAssociated,
            MixsPackage::HumanAssociated,
            MixsPackage::HumanGut,
            MixsPackage::HumanOral,
            MixsPackage::HumanSkin,
            MixsPackage::HumanVaginal,
            MixsPackage::MicrobialMatBiofilm,
            MixsPackage::MiscellaneousNaturalOrArtificialEnvironment,
            MixsPackage::PlantAssociated,
            MixsPackage::Sediment,
            MixsPackage::Soil,
            MixsPackage::WastewaterSludge,
            MixsPackage::Water,
        ]
    }
}

/// A MIxS field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixsField {
    /// Field name (e.g., "lat_lon", "collection_date").
    pub name: String,
    /// Human-readable label.
    pub label: String,
    /// Description of the field.
    pub description: String,
    /// Requirement level.
    pub requirement: MixsFieldRequirement,
    /// Expected format or pattern.
    pub format: Option<String>,
    /// Example value.
    pub example: Option<String>,
    /// Associated ontology (e.g., "ENVO" for environmental terms).
    pub ontology: Option<String>,
    /// Common aliases for this field.
    pub aliases: Vec<String>,
}

impl MixsField {
    /// Create a new MIxS field.
    pub fn new(name: impl Into<String>, requirement: MixsFieldRequirement) -> Self {
        let name = name.into();
        Self {
            label: name.replace('_', " "),
            name,
            description: String::new(),
            requirement,
            format: None,
            example: None,
            ontology: None,
            aliases: Vec::new(),
        }
    }

    /// Set the label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set an example.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Set the ontology.
    pub fn with_ontology(mut self, ontology: impl Into<String>) -> Self {
        self.ontology = Some(ontology.into());
        self
    }

    /// Add aliases.
    pub fn with_aliases(mut self, aliases: Vec<&str>) -> Self {
        self.aliases = aliases.into_iter().map(String::from).collect();
        self
    }

    /// Check if a column name matches this field (including aliases).
    pub fn matches_column(&self, column: &str) -> bool {
        let col_lower = column.to_lowercase().replace(['-', ' '], "_");
        let name_lower = self.name.to_lowercase();

        if col_lower == name_lower {
            return true;
        }

        for alias in &self.aliases {
            if col_lower == alias.to_lowercase().replace(['-', ' '], "_") {
                return true;
            }
        }

        false
    }
}

/// MIxS schema containing field definitions.
#[derive(Debug, Clone)]
pub struct MixsSchema {
    /// Core fields required for all MIxS submissions.
    pub core_fields: Vec<MixsField>,
    /// Package-specific fields.
    pub package_fields: HashMap<MixsPackage, Vec<MixsField>>,
}

impl MixsSchema {
    /// Create a new MIxS schema with default definitions.
    pub fn new() -> Self {
        let mut schema = Self {
            core_fields: Vec::new(),
            package_fields: HashMap::new(),
        };
        schema.load_core_fields();
        schema.load_package_fields();
        schema
    }

    /// Load MIxS core field definitions.
    fn load_core_fields(&mut self) {
        self.core_fields = vec![
            MixsField::new("investigation_type", MixsFieldRequirement::Mandatory)
                .with_label("Investigation Type")
                .with_description("Type of investigation (e.g., metagenome, genome, transcriptome)")
                .with_example("metagenome"),
            MixsField::new("project_name", MixsFieldRequirement::Mandatory)
                .with_label("Project Name")
                .with_description("Name of the project")
                .with_aliases(vec!["study_name", "project", "study"]),
            MixsField::new("lat_lon", MixsFieldRequirement::Mandatory)
                .with_label("Latitude/Longitude")
                .with_description("Geographic coordinates (decimal degrees)")
                .with_format("DD.DDDDDD DD.DDDDDD or DD.DDDDDD N/S DD.DDDDDD E/W")
                .with_example("38.98 -77.11")
                .with_aliases(vec!["latitude_longitude", "geo_loc", "coordinates"]),
            MixsField::new("geo_loc_name", MixsFieldRequirement::Mandatory)
                .with_label("Geographic Location")
                .with_description("Geographic location (country and/or region)")
                .with_format("Country: Region")
                .with_example("USA: Maryland")
                .with_aliases(vec!["geographic_location", "location", "country"]),
            MixsField::new("collection_date", MixsFieldRequirement::Mandatory)
                .with_label("Collection Date")
                .with_description("Date when sample was collected")
                .with_format("YYYY-MM-DD or YYYY-MM or YYYY")
                .with_example("2024-01-15")
                .with_aliases(vec!["sample_date", "date_collected", "sampling_date"]),
            MixsField::new("env_broad_scale", MixsFieldRequirement::Mandatory)
                .with_label("Broad-scale Environmental Context")
                .with_description("Biome or major environment type (ENVO term)")
                .with_ontology("ENVO")
                .with_example("ENVO:01000253 (freshwater biome)")
                .with_aliases(vec!["biome", "environment_biome", "env_biome"]),
            MixsField::new("env_local_scale", MixsFieldRequirement::Mandatory)
                .with_label("Local Environmental Context")
                .with_description("Local environmental feature (ENVO term)")
                .with_ontology("ENVO")
                .with_example("ENVO:00000067 (cave)")
                .with_aliases(vec!["feature", "environment_feature", "env_feature"]),
            MixsField::new("env_medium", MixsFieldRequirement::Mandatory)
                .with_label("Environmental Medium")
                .with_description("Material from which sample was obtained (ENVO term)")
                .with_ontology("ENVO")
                .with_example("ENVO:00002007 (sediment)")
                .with_aliases(vec!["material", "environment_material", "env_material", "sample_material"]),
            MixsField::new("seq_meth", MixsFieldRequirement::Recommended)
                .with_label("Sequencing Method")
                .with_description("Sequencing method/platform used")
                .with_example("Illumina HiSeq 2500")
                .with_aliases(vec!["sequencing_method", "platform", "seq_platform"]),
            MixsField::new("samp_size", MixsFieldRequirement::Recommended)
                .with_label("Sample Size")
                .with_description("Amount of sample collected")
                .with_format("number unit")
                .with_example("5 gram")
                .with_aliases(vec!["sample_size", "amount"]),
        ];
    }

    /// Load package-specific field definitions.
    fn load_package_fields(&mut self) {
        // Human-gut specific fields
        self.package_fields.insert(
            MixsPackage::HumanGut,
            vec![
                MixsField::new("host_subject_id", MixsFieldRequirement::Mandatory)
                    .with_label("Host Subject ID")
                    .with_description("Unique identifier for the human subject")
                    .with_aliases(vec!["subject_id", "patient_id", "participant_id"]),
                MixsField::new("host_age", MixsFieldRequirement::Recommended)
                    .with_label("Host Age")
                    .with_description("Age of the host at time of sampling")
                    .with_format("number unit")
                    .with_example("35 years")
                    .with_aliases(vec!["age", "subject_age"]),
                MixsField::new("host_sex", MixsFieldRequirement::Recommended)
                    .with_label("Host Sex")
                    .with_description("Sex of the host")
                    .with_example("female")
                    .with_aliases(vec!["sex", "gender"]),
                MixsField::new("host_disease_stat", MixsFieldRequirement::Recommended)
                    .with_label("Host Disease Status")
                    .with_description("Disease status of the host")
                    .with_ontology("MONDO")
                    .with_aliases(vec!["disease", "diagnosis", "health_status"]),
                MixsField::new("host_body_site", MixsFieldRequirement::Mandatory)
                    .with_label("Host Body Site")
                    .with_description("Body site from which sample was taken")
                    .with_ontology("UBERON")
                    .with_example("UBERON:0000160 (intestine)")
                    .with_aliases(vec!["body_site", "sample_site", "tissue"]),
                MixsField::new("samp_collect_device", MixsFieldRequirement::Recommended)
                    .with_label("Sample Collection Device")
                    .with_description("Device used to collect sample")
                    .with_example("swab"),
                MixsField::new("gastrointest_disord", MixsFieldRequirement::Conditional)
                    .with_label("Gastrointestinal Disorder")
                    .with_description("History of GI disorders")
                    .with_aliases(vec!["gi_disorder", "gastrointestinal"]),
            ],
        );

        // Soil specific fields
        self.package_fields.insert(
            MixsPackage::Soil,
            vec![
                MixsField::new("depth", MixsFieldRequirement::Mandatory)
                    .with_label("Depth")
                    .with_description("Depth from which sample was collected")
                    .with_format("number unit")
                    .with_example("10 cm"),
                MixsField::new("elev", MixsFieldRequirement::Mandatory)
                    .with_label("Elevation")
                    .with_description("Elevation of sampling site")
                    .with_format("number unit")
                    .with_example("100 m"),
                MixsField::new("soil_type", MixsFieldRequirement::Recommended)
                    .with_label("Soil Type")
                    .with_description("Classification of soil type")
                    .with_example("sandy loam"),
                MixsField::new("ph", MixsFieldRequirement::Recommended)
                    .with_label("pH")
                    .with_description("pH of the soil")
                    .with_example("6.5"),
            ],
        );

        // Water specific fields
        self.package_fields.insert(
            MixsPackage::Water,
            vec![
                MixsField::new("depth", MixsFieldRequirement::Mandatory)
                    .with_label("Depth")
                    .with_description("Depth from which sample was collected")
                    .with_format("number unit")
                    .with_example("10 m"),
                MixsField::new("temp", MixsFieldRequirement::Recommended)
                    .with_label("Temperature")
                    .with_description("Water temperature")
                    .with_format("number unit")
                    .with_example("15 degree Celsius"),
                MixsField::new("salinity", MixsFieldRequirement::Recommended)
                    .with_label("Salinity")
                    .with_description("Salinity of water")
                    .with_format("number unit")
                    .with_example("35 psu"),
            ],
        );

        // Add empty entries for other packages (to be expanded)
        for package in MixsPackage::all() {
            self.package_fields.entry(*package).or_insert_with(Vec::new);
        }
    }

    /// Get all fields for a specific package (core + package-specific).
    pub fn fields_for_package(&self, package: MixsPackage) -> Vec<&MixsField> {
        let mut fields: Vec<&MixsField> = self.core_fields.iter().collect();
        if let Some(pkg_fields) = self.package_fields.get(&package) {
            fields.extend(pkg_fields.iter());
        }
        fields
    }

    /// Get only mandatory fields for a package.
    pub fn mandatory_fields_for_package(&self, package: MixsPackage) -> Vec<&MixsField> {
        self.fields_for_package(package)
            .into_iter()
            .filter(|f| f.requirement == MixsFieldRequirement::Mandatory)
            .collect()
    }

    /// Find a field by column name (checks aliases).
    pub fn find_field(&self, column: &str, package: Option<MixsPackage>) -> Option<&MixsField> {
        // Check core fields first
        for field in &self.core_fields {
            if field.matches_column(column) {
                return Some(field);
            }
        }

        // Check package-specific fields if package is specified
        if let Some(pkg) = package {
            if let Some(pkg_fields) = self.package_fields.get(&pkg) {
                for field in pkg_fields {
                    if field.matches_column(column) {
                        return Some(field);
                    }
                }
            }
        }

        None
    }
}

impl Default for MixsSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Static reference to core MIxS fields (for quick access).
pub static MIXS_CORE_FIELDS: &[&str] = &[
    "investigation_type",
    "project_name",
    "lat_lon",
    "geo_loc_name",
    "collection_date",
    "env_broad_scale",
    "env_local_scale",
    "env_medium",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_from_string() {
        assert_eq!(
            MixsPackage::from_str_flexible("human-gut"),
            Some(MixsPackage::HumanGut)
        );
        assert_eq!(
            MixsPackage::from_str_flexible("gut"),
            Some(MixsPackage::HumanGut)
        );
        assert_eq!(
            MixsPackage::from_str_flexible("SOIL"),
            Some(MixsPackage::Soil)
        );
        assert_eq!(
            MixsPackage::from_str_flexible("stool"),
            Some(MixsPackage::HumanGut)
        );
    }

    #[test]
    fn test_field_matching() {
        let field = MixsField::new("collection_date", MixsFieldRequirement::Mandatory)
            .with_aliases(vec!["sample_date", "date_collected"]);

        assert!(field.matches_column("collection_date"));
        assert!(field.matches_column("Collection_Date"));
        assert!(field.matches_column("sample_date"));
        assert!(field.matches_column("date_collected"));
        assert!(!field.matches_column("some_other_field"));
    }

    #[test]
    fn test_schema_core_fields() {
        let schema = MixsSchema::new();
        assert!(!schema.core_fields.is_empty());

        // Check that lat_lon is mandatory
        let lat_lon = schema.core_fields.iter().find(|f| f.name == "lat_lon");
        assert!(lat_lon.is_some());
        assert_eq!(lat_lon.unwrap().requirement, MixsFieldRequirement::Mandatory);
    }

    #[test]
    fn test_schema_package_fields() {
        let schema = MixsSchema::new();
        let gut_fields = schema.fields_for_package(MixsPackage::HumanGut);

        // Should have core fields + gut-specific fields
        assert!(gut_fields.len() > schema.core_fields.len());

        // Should have host_subject_id
        let has_subject_id = gut_fields.iter().any(|f| f.name == "host_subject_id");
        assert!(has_subject_id);
    }

    #[test]
    fn test_find_field_with_alias() {
        let schema = MixsSchema::new();

        // Should find collection_date via alias
        let field = schema.find_field("sample_date", None);
        assert!(field.is_some());
        assert_eq!(field.unwrap().name, "collection_date");
    }
}
