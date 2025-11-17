use crate::lowering::HirType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IteratorTrait {
    Iterator,
    IntoIterator,
    FromIterator,
}

impl IteratorTrait {
    pub fn to_string(&self) -> &'static str {
        match self {
            IteratorTrait::Iterator => "Iterator",
            IteratorTrait::IntoIterator => "IntoIterator",
            IteratorTrait::FromIterator => "FromIterator",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IteratorInfo {
    pub item_type: HirType,
    pub container_type: HirType,
    pub trait_impl: IteratorTrait,
}

pub struct IteratorAnalyzer;

impl IteratorAnalyzer {
    pub fn new() -> Self {
        IteratorAnalyzer
    }

    pub fn analyze_iterator_type(ty: &HirType) -> Option<IteratorInfo> {
        match ty {
            HirType::Named(type_name) if type_name == "Vec" => {
                Some(IteratorInfo {
                    item_type: HirType::Unknown,
                    container_type: ty.clone(),
                    trait_impl: IteratorTrait::IntoIterator,
                })
            }
            HirType::Array { element_type, .. } => {
                Some(IteratorInfo {
                    item_type: (**element_type).clone(),
                    container_type: ty.clone(),
                    trait_impl: IteratorTrait::IntoIterator,
                })
            }
            HirType::String => {
                Some(IteratorInfo {
                    item_type: HirType::String,
                    container_type: ty.clone(),
                    trait_impl: IteratorTrait::IntoIterator,
                })
            }
            _ => None,
        }
    }

    pub fn get_iterator_item_type(iterator_type: &HirType) -> Option<HirType> {
        if let HirType::Named(name) = iterator_type {
            if name.starts_with("Iterator<") && name.ends_with(">") {
                let inner_type = name.strip_prefix("Iterator<")
                    .and_then(|s| s.strip_suffix(">"))?;
                
                return match inner_type {
                    "i32" => Some(HirType::Int32),
                    "i64" => Some(HirType::Int64),
                    "f64" => Some(HirType::Float64),
                    "bool" => Some(HirType::Bool),
                    "str" => Some(HirType::String),
                    _ => Some(HirType::Named(inner_type.to_string())),
                };
            }
        }
        None
    }

    pub fn construct_iterator_type(item_type: &HirType) -> HirType {
        match item_type {
            HirType::Int32 => HirType::Named("Iterator<i32>".to_string()),
            HirType::Int64 => HirType::Named("Iterator<i64>".to_string()),
            HirType::Float64 => HirType::Named("Iterator<f64>".to_string()),
            HirType::Bool => HirType::Named("Iterator<bool>".to_string()),
            HirType::String => HirType::Named("Iterator<str>".to_string()),
            HirType::Named(name) => HirType::Named(format!("Iterator<{}>", name)),
            _ => HirType::Unknown,
        }
    }
}

pub struct IteratorMethodHandler;

impl IteratorMethodHandler {
    pub fn is_iterator_method(method_name: &str) -> bool {
        matches!(
            method_name,
            "iter" | "iter_mut" | "into_iter" | "next" | "map" | "filter" | "collect"
        )
    }

    pub fn get_method_signature(
        receiver_type: &HirType,
        method_name: &str,
    ) -> Option<(Vec<HirType>, HirType)> {
        match method_name {
            "iter" => {
                match receiver_type {
                    HirType::Array { element_type, .. } => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(element_type);
                        Some((vec![], iter_type))
                    }
                    HirType::Named(name) if name == "Vec" => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(&HirType::Unknown);
                        Some((vec![], iter_type))
                    }
                    _ => None,
                }
            }
            "iter_mut" => {
                match receiver_type {
                    HirType::Array { element_type, .. } => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(element_type);
                        Some((vec![], iter_type))
                    }
                    HirType::Named(name) if name == "Vec" => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(&HirType::Unknown);
                        Some((vec![], iter_type))
                    }
                    _ => None,
                }
            }
            "into_iter" => {
                match receiver_type {
                    HirType::Array { element_type, .. } => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(element_type);
                        Some((vec![], iter_type))
                    }
                    HirType::Named(name) if name == "Vec" => {
                        let iter_type = IteratorAnalyzer::construct_iterator_type(&HirType::Unknown);
                        Some((vec![], iter_type))
                    }
                    _ => None,
                }
            }
            "next" => {
                if let Some(item_type) = IteratorAnalyzer::get_iterator_item_type(receiver_type) {
                    Some((vec![], HirType::Named(format!("Option<{:?}>", item_type))))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_array_iterator() {
        let array_type = HirType::Array {
            element_type: Box::new(HirType::Int32),
            size: Some(10),
        };
        let info = IteratorAnalyzer::analyze_iterator_type(&array_type);
        assert!(info.is_some());
        assert_eq!(info.unwrap().trait_impl, IteratorTrait::IntoIterator);
    }

    #[test]
    fn test_construct_iterator_type() {
        let iter_type = IteratorAnalyzer::construct_iterator_type(&HirType::Int32);
        match iter_type {
            HirType::Named(name) => assert_eq!(name, "Iterator<i32>"),
            _ => panic!("Expected named type"),
        }
    }

    #[test]
    fn test_get_iterator_item_type() {
        let iter_type = HirType::Named("Iterator<i32>".to_string());
        let item_type = IteratorAnalyzer::get_iterator_item_type(&iter_type);
        assert_eq!(item_type, Some(HirType::Int32));
    }

    #[test]
    fn test_iterator_method_iter_on_array() {
        let array_type = HirType::Array {
            element_type: Box::new(HirType::Int32),
            size: Some(5),
        };
        let sig = IteratorMethodHandler::get_method_signature(&array_type, "iter");
        assert!(sig.is_some());
        let (params, ret) = sig.unwrap();
        assert_eq!(params.len(), 0);
        match ret {
            HirType::Named(name) => assert!(name.contains("Iterator")),
            _ => panic!("Expected iterator type"),
        }
    }

    #[test]
    fn test_iterator_method_recognition() {
        assert!(IteratorMethodHandler::is_iterator_method("iter"));
        assert!(IteratorMethodHandler::is_iterator_method("into_iter"));
        assert!(IteratorMethodHandler::is_iterator_method("map"));
        assert!(!IteratorMethodHandler::is_iterator_method("len"));
    }

    #[test]
    fn test_iterator_trait_to_string() {
        assert_eq!(IteratorTrait::Iterator.to_string(), "Iterator");
        assert_eq!(IteratorTrait::IntoIterator.to_string(), "IntoIterator");
        assert_eq!(IteratorTrait::FromIterator.to_string(), "FromIterator");
    }

    #[test]
    fn test_string_is_iterable() {
        let info = IteratorAnalyzer::analyze_iterator_type(&HirType::String);
        assert!(info.is_some());
        assert_eq!(info.unwrap().trait_impl, IteratorTrait::IntoIterator);
    }

    #[test]
    fn test_iterator_chain_map_filter() {
        let iter_type = HirType::Named("Iterator<i32>".to_string());
        
        assert!(IteratorMethodHandler::is_iterator_method("map"));
        assert!(IteratorMethodHandler::is_iterator_method("filter"));
        
        let item_type = IteratorAnalyzer::get_iterator_item_type(&iter_type);
        assert_eq!(item_type, Some(HirType::Int32));
    }

    #[test]
    fn test_iterator_collect_method() {
        assert!(IteratorMethodHandler::is_iterator_method("collect"));
    }

    #[test]
    fn test_iterator_method_on_vec() {
        let vec_type = HirType::Named("Vec".to_string());
        let info = IteratorAnalyzer::analyze_iterator_type(&vec_type);
        assert!(info.is_some());
        
        let sig = IteratorMethodHandler::get_method_signature(&vec_type, "iter");
        assert!(sig.is_some());
    }

    #[test]
    fn test_nested_iterator_types() {
        let item_type = HirType::Named("Iterator<i32>".to_string());
        let nested_iter = IteratorAnalyzer::construct_iterator_type(&item_type);
        
        match nested_iter {
            HirType::Named(name) => assert!(name.contains("Iterator")),
            _ => panic!("Expected named iterator type"),
        }
    }

    #[test]
    fn test_iterator_signature_consistency() {
        let array_type = HirType::Array {
            element_type: Box::new(HirType::Int32),
            size: Some(5),
        };
        
        let iter_sig = IteratorMethodHandler::get_method_signature(&array_type, "iter");
        let into_iter_sig = IteratorMethodHandler::get_method_signature(&array_type, "into_iter");
        
        assert!(iter_sig.is_some());
        assert!(into_iter_sig.is_some());
        
        let (iter_params, _) = iter_sig.unwrap();
        let (into_params, _) = into_iter_sig.unwrap();
        
        assert_eq!(iter_params.len(), 0);
        assert_eq!(into_params.len(), 0);
    }

    #[test]
    fn test_invalid_method_on_non_iterable() {
        let scalar_type = HirType::Int32;
        let sig = IteratorMethodHandler::get_method_signature(&scalar_type, "iter");
        assert!(sig.is_none());
    }
}
