/// Derive expansion module: Expands #[derive(...)] attributes into impl blocks
/// 
/// This module handles the integration of derive macros into the compilation pipeline
/// by converting derive attributes into proper HirItem::Impl blocks that get compiled.

use crate::lowering::HirItem;
use crate::frontend::derive_macros::DeriveEngine;

/// Expands derive attributes on a struct into impl blocks
/// 
/// Takes a struct HirItem with derives and generates corresponding impl HirItems
/// that will be compiled normally through the pipeline.
/// 
/// Note: For now, this creates minimal impl blocks. The actual method implementations
/// are handled by special code in the codegen phase which recognizes derived traits.
/// 
/// # Arguments
/// * `struct_item` - The struct with derive attributes
/// 
/// # Returns
/// Vector of impl HirItems corresponding to each derived trait
pub fn expand_derives(
    struct_item: &HirItem,
) -> Result<Vec<HirItem>, String> {
    // Extract struct definition
    let (struct_name, _fields, derives, is_public) = match struct_item {
        HirItem::Struct {
            name,
            fields,
            derives,
            is_public,
        } => (name.clone(), fields.clone(), derives.clone(), *is_public),
        _ => return Err("expand_derives called on non-struct item".to_string()),
    };

    if derives.is_empty() {
        return Ok(Vec::new());
    }

    // Convert derives from Vec<String> to Vec<DeriveKind>
    let mut engine = DeriveEngine::new();
    let _derive_kinds = engine.parse_derive_kinds(&derives)
        .map_err(|e| format!("Invalid derive: {:?}", e))?;

    // For now, we create placeholder impl blocks
    // The actual code generation will be handled by the codegen phase,
    // which will recognize these trait impls and generate appropriate code
    let mut impl_items = Vec::new();
    
    for derive_name in derives {
        let hir_impl = HirItem::Impl {
            trait_name: Some(derive_name.clone()),
            struct_name: struct_name.clone(),
            methods: vec![], // Methods populated during codegen
            generics: vec![],
            is_unsafe: false,
            is_public,
        };
        impl_items.push(hir_impl);
    }

    Ok(impl_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::HirType;

    fn create_test_struct() -> HirItem {
        HirItem::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), HirType::Int32),
                ("y".to_string(), HirType::Int32),
            ],
            derives: vec!["Clone".to_string(), "Debug".to_string()],
            is_public: true,
        }
    }

    #[test]
    fn test_expand_derives_creates_impl_blocks() {
        let struct_item = create_test_struct();
        let result = expand_derives(&struct_item);
        
        assert!(result.is_ok(), "expand_derives should succeed");
        let impl_blocks = result.unwrap();
        
        // Should create 2 impl blocks (Clone and Debug)
        assert_eq!(impl_blocks.len(), 2, "Should create impl for Clone and Debug");
    }

    #[test]
    fn test_expand_derives_clone_only() {
        let struct_item = HirItem::Struct {
            name: "Value".to_string(),
            fields: vec![("val".to_string(), HirType::Int32)],
            derives: vec!["Clone".to_string()],
            is_public: false,
        };
        
        let result = expand_derives(&struct_item);
        assert!(result.is_ok());
        
        let impl_blocks = result.unwrap();
        assert_eq!(impl_blocks.len(), 1);
        
        // Check that it's a Clone impl
        if let HirItem::Impl {
            trait_name: Some(trait_name),
            struct_name,
            ..
        } = &impl_blocks[0]
        {
            assert_eq!(trait_name, "Clone");
            assert_eq!(struct_name, "Value");
        } else {
            panic!("Expected HirItem::Impl");
        }
    }

    #[test]
    fn test_expand_derives_empty() {
        let struct_item = HirItem::Struct {
            name: "Empty".to_string(),
            fields: vec![],
            derives: vec![], // No derives
            is_public: true,
        };
        
        let result = expand_derives(&struct_item);
        assert!(result.is_ok());
        
        let impl_blocks = result.unwrap();
        assert_eq!(impl_blocks.len(), 0, "No derives means no impl blocks");
    }

    #[test]
    fn test_expand_derives_invalid_derive() {
        let struct_item = HirItem::Struct {
            name: "Bad".to_string(),
            fields: vec![],
            derives: vec!["InvalidDrive".to_string()],
            is_public: true,
        };
        
        let result = expand_derives(&struct_item);
        assert!(result.is_err(), "Invalid derive should fail");
    }
}
