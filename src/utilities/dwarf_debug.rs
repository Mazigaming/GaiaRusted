
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum DwarfTag {
    CompileUnit,
    SubProgram,
    Variable,
    Parameter,
    BaseType,
    PointerType,
    ArrayType,
    StructType,
    UnionType,
    Enumeration,
    LexicalBlock,
}

#[derive(Debug, Clone)]
pub struct DwarfAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct DwarfDie {
    pub tag: DwarfTag,
    pub attributes: HashMap<String, DwarfAttribute>,
    pub offset: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DwarfStats {
    pub compile_units: usize,
    pub subprograms: usize,
    pub variables: usize,
    pub lines_generated: usize,
}

pub struct DwarfGenerator {
    dies: Vec<DwarfDie>,
    line_info: LineInfo,
    statistics: DwarfStats,
}

pub struct LineInfo {
    pub files: Vec<String>,
    pub lines: Vec<LineEntry>,
}

#[derive(Debug, Clone)]
pub struct LineEntry {
    pub file_idx: usize,
    pub line: u32,
    pub column: u32,
    pub address: u64,
}

impl DwarfGenerator {
    pub fn new() -> Self {
        DwarfGenerator {
            dies: Vec::new(),
            line_info: LineInfo {
                files: Vec::new(),
                lines: Vec::new(),
            },
            statistics: DwarfStats {
                compile_units: 0,
                subprograms: 0,
                variables: 0,
                lines_generated: 0,
            },
        }
    }

    pub fn add_compile_unit(&mut self, name: &str, producer: &str) -> u64 {
        let mut attrs = HashMap::new();
        attrs.insert(
            "name".to_string(),
            DwarfAttribute {
                name: "name".to_string(),
                value: name.to_string(),
            },
        );
        attrs.insert(
            "producer".to_string(),
            DwarfAttribute {
                name: "producer".to_string(),
                value: producer.to_string(),
            },
        );
        attrs.insert(
            "language".to_string(),
            DwarfAttribute {
                name: "language".to_string(),
                value: "Rust".to_string(),
            },
        );

        let offset = self.dies.len() as u64;
        self.dies.push(DwarfDie {
            tag: DwarfTag::CompileUnit,
            attributes: attrs,
            offset,
        });

        self.statistics.compile_units += 1;
        offset
    }

    pub fn add_subprogram(&mut self, name: &str, file: &str, line: u32, address: u64) -> u64 {
        let mut attrs = HashMap::new();
        attrs.insert(
            "name".to_string(),
            DwarfAttribute {
                name: "name".to_string(),
                value: name.to_string(),
            },
        );
        attrs.insert(
            "file".to_string(),
            DwarfAttribute {
                name: "file".to_string(),
                value: file.to_string(),
            },
        );
        attrs.insert(
            "line".to_string(),
            DwarfAttribute {
                name: "line".to_string(),
                value: line.to_string(),
            },
        );
        attrs.insert(
            "address".to_string(),
            DwarfAttribute {
                name: "address".to_string(),
                value: format!("0x{:x}", address),
            },
        );

        let offset = self.dies.len() as u64;
        self.dies.push(DwarfDie {
            tag: DwarfTag::SubProgram,
            attributes: attrs,
            offset,
        });

        self.statistics.subprograms += 1;
        offset
    }

    pub fn add_variable(&mut self, name: &str, var_type: &str, location: &str) -> u64 {
        let mut attrs = HashMap::new();
        attrs.insert(
            "name".to_string(),
            DwarfAttribute {
                name: "name".to_string(),
                value: name.to_string(),
            },
        );
        attrs.insert(
            "type".to_string(),
            DwarfAttribute {
                name: "type".to_string(),
                value: var_type.to_string(),
            },
        );
        attrs.insert(
            "location".to_string(),
            DwarfAttribute {
                name: "location".to_string(),
                value: location.to_string(),
            },
        );

        let offset = self.dies.len() as u64;
        self.dies.push(DwarfDie {
            tag: DwarfTag::Variable,
            attributes: attrs,
            offset,
        });

        self.statistics.variables += 1;
        offset
    }

    pub fn add_file(&mut self, file_path: &str) -> usize {
        let idx = self.line_info.files.len();
        self.line_info.files.push(file_path.to_string());
        idx
    }

    pub fn add_line_entry(&mut self, file_idx: usize, line: u32, column: u32, address: u64) {
        self.line_info.lines.push(LineEntry {
            file_idx,
            line,
            column,
            address,
        });
        self.statistics.lines_generated += 1;
    }

    pub fn generate_debug_info(&self) -> String {
        let mut debug_info = String::new();

        debug_info.push_str(".section .debug_info\n");
        for die in &self.dies {
            debug_info.push_str(&self.format_die(die));
        }

        debug_info
    }

    fn format_die(&self, die: &DwarfDie) -> String {
        let mut result = String::new();
        result.push_str(&format!("  ; DIE offset 0x{:x}\n", die.offset));

        let tag_name = match die.tag {
            DwarfTag::CompileUnit => "DW_TAG_compile_unit",
            DwarfTag::SubProgram => "DW_TAG_subprogram",
            DwarfTag::Variable => "DW_TAG_variable",
            DwarfTag::Parameter => "DW_TAG_formal_parameter",
            DwarfTag::BaseType => "DW_TAG_base_type",
            DwarfTag::PointerType => "DW_TAG_pointer_type",
            DwarfTag::ArrayType => "DW_TAG_array_type",
            DwarfTag::StructType => "DW_TAG_structure_type",
            DwarfTag::UnionType => "DW_TAG_union_type",
            DwarfTag::Enumeration => "DW_TAG_enumeration_type",
            DwarfTag::LexicalBlock => "DW_TAG_lexical_block",
        };

        result.push_str(&format!("  .uleb128 1                  ; (DIE (0x{:x}) {})\n", die.offset, tag_name));

        for (_, attr) in &die.attributes {
            result.push_str(&format!("  ; {}={}\n", attr.name, attr.value));
        }

        result.push_str("\n");
        result
    }

    pub fn generate_line_info(&self) -> String {
        let mut line_info = String::new();

        line_info.push_str(".section .debug_line\n");

        for (idx, file) in self.line_info.files.iter().enumerate() {
            line_info.push_str(&format!("  ; File {}: {}\n", idx, file));
        }

        line_info.push_str("\n  ; Line entries:\n");
        for entry in &self.line_info.lines {
            if entry.file_idx < self.line_info.files.len() {
                let file = &self.line_info.files[entry.file_idx];
                line_info.push_str(&format!(
                    "  ; 0x{:x} {}:{}:{}\n",
                    entry.address, file, entry.line, entry.column
                ));
            }
        }

        line_info
    }

    pub fn generate_aranges(&self) -> String {
        let mut aranges = String::new();

        aranges.push_str(".section .debug_aranges\n");
        aranges.push_str("  .long .Ldebug_aranges_end - .Ldebug_aranges_start\n");
        aranges.push_str(".Ldebug_aranges_start:\n");
        aranges.push_str("  .short 2                    ; version\n");
        aranges.push_str("  .long .Ldebug_info_start    ; debug_info offset\n");
        aranges.push_str("  .byte 8                     ; address size\n");
        aranges.push_str("  .byte 0                     ; segment size\n");

        aranges
    }

    pub fn get_statistics(&self) -> DwarfStats {
        self.statistics
    }

    pub fn dump(&self) -> String {
        let mut output = String::new();

        output.push_str("=== DWARF Debug Information ===\n\n");
        output.push_str(&format!("Compile Units: {}\n", self.statistics.compile_units));
        output.push_str(&format!("Subprograms: {}\n", self.statistics.subprograms));
        output.push_str(&format!("Variables: {}\n", self.statistics.variables));
        output.push_str(&format!("Line Entries: {}\n\n", self.statistics.lines_generated));

        output.push_str("DIEs:\n");
        for die in &self.dies {
            output.push_str(&format!("  Offset 0x{:x}: {:?}\n", die.offset, die.tag));
            for (_, attr) in &die.attributes {
                output.push_str(&format!("    {} = {}\n", attr.name, attr.value));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwarf_generator_creation() {
        let _gen = DwarfGenerator::new();
    }

    #[test]
    fn test_add_compile_unit() {
        let mut gen = DwarfGenerator::new();
        let offset = gen.add_compile_unit("test.rs", "GaiaRusted");
        assert_eq!(offset, 0);
        assert_eq!(gen.get_statistics().compile_units, 1);
    }

    #[test]
    fn test_add_subprogram() {
        let mut gen = DwarfGenerator::new();
        gen.add_compile_unit("test.rs", "GaiaRusted");
        let offset = gen.add_subprogram("main", "test.rs", 1, 0x400);
        assert_eq!(offset, 1);
        assert_eq!(gen.get_statistics().subprograms, 1);
    }

    #[test]
    fn test_add_variable() {
        let mut gen = DwarfGenerator::new();
        gen.add_compile_unit("test.rs", "GaiaRusted");
        let offset = gen.add_variable("x", "i64", "%rbp-8");
        assert_eq!(offset, 1);
        assert_eq!(gen.get_statistics().variables, 1);
    }

    #[test]
    fn test_add_file() {
        let mut gen = DwarfGenerator::new();
        let idx1 = gen.add_file("test.rs");
        let idx2 = gen.add_file("lib.rs");
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
    }

    #[test]
    fn test_add_line_entry() {
        let mut gen = DwarfGenerator::new();
        gen.add_file("test.rs");
        gen.add_line_entry(0, 1, 0, 0x400);
        assert_eq!(gen.get_statistics().lines_generated, 1);
    }

    #[test]
    fn test_generate_debug_info() {
        let mut gen = DwarfGenerator::new();
        gen.add_compile_unit("test.rs", "GaiaRusted");
        let debug_info = gen.generate_debug_info();
        assert!(debug_info.contains(".section .debug_info"));
    }

    #[test]
    fn test_generate_line_info() {
        let mut gen = DwarfGenerator::new();
        gen.add_file("test.rs");
        gen.add_line_entry(0, 1, 0, 0x400);
        let line_info = gen.generate_line_info();
        assert!(line_info.contains(".section .debug_line"));
    }
}
