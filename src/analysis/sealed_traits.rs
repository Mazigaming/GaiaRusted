use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SealedTrait {
    pub name: String,
    pub allowed_types: HashSet<String>,
    pub sealing_module: String,
}

#[derive(Debug, Clone)]
pub struct SealingMarker {
    pub trait_name: String,
    pub marker_name: String,
}

pub struct SealedTraitManager {
    sealed_traits: std::collections::HashMap<String, SealedTrait>,
    markers: Vec<SealingMarker>,
}

impl SealedTraitManager {
    pub fn new() -> Self {
        SealedTraitManager {
            sealed_traits: std::collections::HashMap::new(),
            markers: Vec::new(),
        }
    }

    pub fn seal_trait(
        &mut self,
        trait_name: String,
        allowed_types: HashSet<String>,
        sealing_module: String,
    ) -> Result<(), String> {
        if self.sealed_traits.contains_key(&trait_name) {
            return Err(format!("Trait {} is already sealed", trait_name));
        }

        let sealed_trait = SealedTrait {
            name: trait_name.clone(),
            allowed_types,
            sealing_module,
        };

        self.sealed_traits.insert(trait_name.clone(), sealed_trait);

        let marker = SealingMarker {
            trait_name: trait_name.clone(),
            marker_name: format!("{}Sealed", trait_name),
        };
        self.markers.push(marker);

        Ok(())
    }

    pub fn can_implement(&self, trait_name: &str, implementing_type: &str) -> bool {
        if let Some(sealed_trait) = self.sealed_traits.get(trait_name) {
            sealed_trait.allowed_types.contains(implementing_type)
        } else {
            true
        }
    }

    pub fn is_sealed(&self, trait_name: &str) -> bool {
        self.sealed_traits.contains_key(trait_name)
    }

    pub fn get_allowed_types(&self, trait_name: &str) -> Option<Vec<String>> {
        self.sealed_traits
            .get(trait_name)
            .map(|t| t.allowed_types.iter().cloned().collect())
    }

    pub fn add_allowed_type(&mut self, trait_name: &str, type_name: String) -> Result<(), String> {
        if let Some(sealed_trait) = self.sealed_traits.get_mut(trait_name) {
            sealed_trait.allowed_types.insert(type_name);
            Ok(())
        } else {
            Err(format!("Trait {} is not sealed", trait_name))
        }
    }

    pub fn remove_allowed_type(
        &mut self,
        trait_name: &str,
        type_name: &str,
    ) -> Result<(), String> {
        if let Some(sealed_trait) = self.sealed_traits.get_mut(trait_name) {
            sealed_trait.allowed_types.remove(type_name);
            Ok(())
        } else {
            Err(format!("Trait {} is not sealed", trait_name))
        }
    }

    pub fn get_sealing_module(&self, trait_name: &str) -> Option<String> {
        self.sealed_traits
            .get(trait_name)
            .map(|t| t.sealing_module.clone())
    }

    pub fn generate_sealing_code(trait_name: &str, allowed_types: &[&str]) -> String {
        let mut code = format!(
            "// Sealed trait: {} can only be implemented by the listed types\n",
            trait_name
        );
        code.push_str("// This is enforced through a private marker type pattern\n");
        code.push_str(&format!("pub trait {} {{\n", trait_name));
        code.push_str("    fn method(&self);\n");
        code.push_str("}\n\n");

        code.push_str(&format!("mod sealed_{} {{\n", trait_name.to_lowercase()));
        code.push_str(&format!("    pub trait Sealed {{}}\n"));

        for allowed_type in allowed_types {
            code.push_str(&format!("    impl Sealed for {} {{}}\n", allowed_type));
        }

        code.push_str("}\n\n");

        for allowed_type in allowed_types {
            code.push_str(&format!(
                "impl {} for {} {{\n",
                trait_name, allowed_type
            ));
            code.push_str("    fn method(&self) {\n");
            code.push_str("        // Implementation\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }

        code
    }

    pub fn validate_implementation(
        &self,
        trait_name: &str,
        type_name: &str,
    ) -> Result<(), String> {
        if !self.can_implement(trait_name, type_name) {
            return Err(format!(
                "Type {} cannot implement sealed trait {}",
                type_name, trait_name
            ));
        }
        Ok(())
    }

    pub fn get_all_sealed_traits(&self) -> Vec<String> {
        self.sealed_traits.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_trait() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("i32".to_string());
        allowed.insert("String".to_string());

        let result = manager.seal_trait(
            "MyTrait".to_string(),
            allowed,
            "my_module".to_string(),
        );
        assert!(result.is_ok());
        assert!(manager.is_sealed("MyTrait"));
    }

    #[test]
    fn test_can_implement() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("Point".to_string());

        manager
            .seal_trait("Display".to_string(), allowed, "std".to_string())
            .unwrap();

        assert!(manager.can_implement("Display", "Point"));
        assert!(!manager.can_implement("Display", "OtherType"));
    }

    #[test]
    fn test_unsealed_trait() {
        let manager = SealedTraitManager::new();
        assert!(!manager.is_sealed("UnknownTrait"));
        assert!(manager.can_implement("UnknownTrait", "AnyType"));
    }

    #[test]
    fn test_get_allowed_types() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("A".to_string());
        allowed.insert("B".to_string());

        manager
            .seal_trait("Trait1".to_string(), allowed, "module".to_string())
            .unwrap();

        let types = manager.get_allowed_types("Trait1");
        assert!(types.is_some());
        let types = types.unwrap();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"A".to_string()));
        assert!(types.contains(&"B".to_string()));
    }

    #[test]
    fn test_add_allowed_type() {
        let mut manager = SealedTraitManager::new();
        let allowed = HashSet::new();
        manager
            .seal_trait("Trait2".to_string(), allowed, "module".to_string())
            .unwrap();

        let result = manager.add_allowed_type("Trait2", "NewType".to_string());
        assert!(result.is_ok());
        assert!(manager.can_implement("Trait2", "NewType"));
    }

    #[test]
    fn test_validate_implementation() {
        let mut manager = SealedTraitManager::new();
        let mut allowed = HashSet::new();
        allowed.insert("AllowedType".to_string());

        manager
            .seal_trait("Trait3".to_string(), allowed, "module".to_string())
            .unwrap();

        assert!(manager
            .validate_implementation("Trait3", "AllowedType")
            .is_ok());
        assert!(manager
            .validate_implementation("Trait3", "DisallowedType")
            .is_err());
    }

    #[test]
    fn test_generate_sealing_code() {
        let code =
            SealedTraitManager::generate_sealing_code("MyTrait", &["Type1", "Type2", "Type3"]);
        assert!(code.contains("MyTrait"));
        assert!(code.contains("Type1"));
        assert!(code.contains("Type2"));
        assert!(code.contains("Type3"));
        assert!(code.contains("Sealed"));
    }
}
