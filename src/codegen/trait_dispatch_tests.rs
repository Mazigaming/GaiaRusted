//! Comprehensive Tests for Trait Objects and Dynamic Dispatch

#[cfg(test)]
mod trait_object_tests {
    use crate::codegen::vtable_generation::VtableGenerator;
    use crate::codegen::dynamic_dispatch::DynamicDispatchCodegen;
    use crate::typesystem::trait_objects::{TraitObject, FatPointer, ObjectSafetyValidator};

    #[test]
    fn test_simple_trait_object() {
        let obj = TraitObject::new("Pet".to_string());
        assert_eq!(obj.trait_name, "Pet");
        assert!(obj.is_object_safe(&[]));
    }

    #[test]
    fn test_vtable_for_simple_trait() {
        let mut gen = VtableGenerator::new();
        let layout = gen.generate_vtable("Pet", "Dog", vec!["speak".to_string()]);

        assert_eq!(layout.trait_name, "Pet");
        assert_eq!(layout.concrete_type, "Dog");
        assert_eq!(layout.entries.len(), 1);
        assert_eq!(layout.size, 8);
    }

    #[test]
    fn test_vtable_multiple_methods() {
        let mut gen = VtableGenerator::new();
        let methods = vec![
            "speak".to_string(),
            "move_".to_string(),
            "describe".to_string(),
        ];
        let layout = gen.generate_vtable("Animal", "Dog", methods);

        assert_eq!(layout.entries.len(), 3);
        assert_eq!(layout.size, 24); // 3 methods * 8 bytes
    }

    #[test]
    fn test_method_offset_calculation() {
        let mut gen = VtableGenerator::new();
        let methods = vec![
            "method1".to_string(),
            "method2".to_string(),
            "method3".to_string(),
        ];
        let layout = gen.generate_vtable("Trait", "Type", methods);

        // Verify offsets are correct
        assert_eq!(layout.entries[0].offset, 0);
        assert_eq!(layout.entries[1].offset, 8);
        assert_eq!(layout.entries[2].offset, 16);
    }

    #[test]
    fn test_dispatch_code_generation() {
        let mut gen = VtableGenerator::new();
        let layout = gen.generate_vtable("Show", "String", vec!["display".to_string()]);

        let code = DynamicDispatchCodegen::generate_trait_method_call(&layout, "display", "rdi", "rax");
        assert!(code.is_some());

        let asm = code.unwrap();
        assert!(asm.contains("mov rax, [rdi + 8]"));
        assert!(asm.contains("call rbx"));
    }

    #[test]
    fn test_fat_pointer_construction() {
        let fp = FatPointer::new("String".to_string(), TraitObject::new("Display".to_string()));
        assert_eq!(fp.data_type, "String");
        assert_eq!(fp.trait_obj.trait_name, "Display");
        assert!(!fp.is_mutable);
    }

    #[test]
    fn test_mutable_fat_pointer() {
        let fp = FatPointer::mutable("Vec".to_string(), TraitObject::new("Iter".to_string()));
        assert!(fp.is_mutable);
    }

    #[test]
    fn test_fat_pointer_size_and_alignment() {
        assert_eq!(FatPointer::size(), 16);
        assert_eq!(FatPointer::alignment(), 8);
    }

    #[test]
    fn test_object_safety_validation() {
        let mut validator = ObjectSafetyValidator::new();

        assert!(validator.validate_trait("Display"));
        assert!(validator.validate_trait("Clone"));
        assert!(validator.validate_trait("Debug"));

        let safe_traits = validator.get_object_safe_traits();
        assert_eq!(safe_traits.len(), 3);
    }

    #[test]
    fn test_multiple_vtables_for_same_trait() {
        let mut gen = VtableGenerator::new();

        // Create vtable for String implementing Display
        let layout1 = gen.generate_vtable("Display", "String", vec!["fmt".to_string()]);
        
        // Create vtable for Vec implementing Display
        let layout2 = gen.generate_vtable("Display", "Vec", vec!["fmt".to_string()]);

        assert_eq!(layout1.concrete_type, "String");
        assert_eq!(layout2.concrete_type, "Vec");
        assert_eq!(layout1.trait_name, layout2.trait_name);
    }

    #[test]
    fn test_assembly_generation() {
        let mut gen = VtableGenerator::new();
        let layout = gen.generate_vtable("Trait", "Type", vec!["method".to_string()]);

        let asm = gen.generate_assembly(&layout);
        assert!(asm.contains(".align 8"));
        assert!(asm.contains(".globl"));
        assert!(asm.contains(".quad"));
    }

    #[test]
    fn test_complex_trait_object_scenario() {
        // Simulate: let x: Box<dyn Iterator> = Box::new(vec![1,2,3].into_iter());
        let mut gen = VtableGenerator::new();
        
        // Iterator trait with multiple methods
        let methods = vec![
            "next".to_string(),
            "size_hint".to_string(),
            "count".to_string(),
        ];
        
        let layout = gen.generate_vtable("Iterator", "VecIter", methods);

        // Create fat pointer
        let _fp = FatPointer::new(
            "VecIter".to_string(),
            TraitObject::new("Iterator".to_string())
        );

        // Generate dispatch code
        let next_code = DynamicDispatchCodegen::generate_trait_method_call(&layout, "next", "rdi", "rax");
        assert!(next_code.is_some());
    }
}
