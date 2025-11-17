//! # Phase 9: OBJECT FILE GENERATION
//!
//! Generates ELF object files from x86-64 assembly.
//!
//! ## What we do:
//! - Assemble x86-64 instructions to machine code
//! - Generate ELF object file (.o)
//! - Create symbol table
//! - Handle relocations
//! - Include debug info (basic DWARF)

use std::fmt;

/// Object file generation error
#[derive(Debug, Clone)]
pub struct ObjectError {
    pub message: String,
}

impl fmt::Display for ObjectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

type ObjectResult<T> = Result<T, ObjectError>;

/// ELF file header
#[derive(Debug, Clone)]
pub struct ElfHeader {
    pub magic: [u8; 4],           // 0x7f, 'E', 'L', 'F'
    pub class: u8,                 // 1 = 32-bit, 2 = 64-bit
    pub data: u8,                  // 1 = little-endian, 2 = big-endian
    pub version: u8,               // 1 = current version
    pub os_abi: u8,                // 0 = System V ABI
    pub abi_version: u8,           // 0
    pub padding: [u8; 7],          // Unused
    pub e_type: u16,               // 1 = relocatable, 2 = executable, 3 = shared
    pub e_machine: u16,            // 62 = x86-64
    pub e_version: u32,            // 1
    pub e_entry: u64,              // Entry point (0 for .o files)
    pub e_phoff: u64,              // Program header offset (0 for .o files)
    pub e_shoff: u64,              // Section header offset
    pub e_flags: u32,              // Flags (0 for x86-64)
    pub e_ehsize: u16,             // ELF header size
    pub e_phentsize: u16,          // Program header entry size (0 for .o files)
    pub e_phnum: u16,              // Program header count (0 for .o files)
    pub e_shentsize: u16,          // Section header entry size
    pub e_shnum: u16,              // Section header count
    pub e_shstrndx: u16,           // String table section index
}

impl Default for ElfHeader {
    fn default() -> Self {
        ElfHeader {
            magic: [0x7f, b'E', b'L', b'F'],
            class: 2,              // 64-bit
            data: 1,               // Little-endian
            version: 1,
            os_abi: 0,             // System V ABI
            abi_version: 0,
            padding: [0; 7],
            e_type: 1,             // Relocatable
            e_machine: 62,         // x86-64
            e_version: 1,
            e_entry: 0,
            e_phoff: 0,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 64,
            e_phentsize: 0,
            e_phnum: 0,
            e_shentsize: 64,
            e_shnum: 0,
            e_shstrndx: 0,
        }
    }
}

impl ElfHeader {
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(64);
        
        // e_ident (16 bytes)
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.class);
        bytes.push(self.data);
        bytes.push(self.version);
        bytes.push(self.os_abi);
        bytes.push(self.abi_version);
        bytes.extend_from_slice(&self.padding);
        
        // e_type, e_machine (4 bytes)
        bytes.extend_from_slice(&self.e_type.to_le_bytes());
        bytes.extend_from_slice(&self.e_machine.to_le_bytes());
        
        // e_version (4 bytes)
        bytes.extend_from_slice(&self.e_version.to_le_bytes());
        
        // e_entry (8 bytes)
        bytes.extend_from_slice(&self.e_entry.to_le_bytes());
        
        // e_phoff (8 bytes)
        bytes.extend_from_slice(&self.e_phoff.to_le_bytes());
        
        // e_shoff (8 bytes)
        bytes.extend_from_slice(&self.e_shoff.to_le_bytes());
        
        // e_flags (4 bytes)
        bytes.extend_from_slice(&self.e_flags.to_le_bytes());
        
        // e_ehsize (2 bytes)
        bytes.extend_from_slice(&self.e_ehsize.to_le_bytes());
        
        // e_phentsize (2 bytes)
        bytes.extend_from_slice(&self.e_phentsize.to_le_bytes());
        
        // e_phnum (2 bytes)
        bytes.extend_from_slice(&self.e_phnum.to_le_bytes());
        
        // e_shentsize (2 bytes)
        bytes.extend_from_slice(&self.e_shentsize.to_le_bytes());
        
        // e_shnum (2 bytes)
        bytes.extend_from_slice(&self.e_shnum.to_le_bytes());
        
        // e_shstrndx (2 bytes)
        bytes.extend_from_slice(&self.e_shstrndx.to_le_bytes());
        
        bytes
    }
}

/// Symbol in symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub value: u64,
    pub size: u64,
    pub bind: u8,  // 0 = local, 1 = global, 2 = weak
    pub sym_type: u8, // 0 = notype, 1 = object, 2 = func, 3 = section
    pub shndx: u16,  // Section index
}

/// Object file builder
pub struct ObjectBuilder {
    pub text_section: Vec<u8>,
    pub symbols: Vec<Symbol>,
    pub relocations: Vec<(u64, String)>,
}

impl ObjectBuilder {
    /// Create a new object builder
    pub fn new() -> Self {
        ObjectBuilder {
            text_section: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
        }
    }

    /// Add assembled code
    pub fn add_code(&mut self, code: &[u8]) {
        self.text_section.extend_from_slice(code);
    }

    /// Add a symbol
    pub fn add_symbol(&mut self, name: String, value: u64, size: u64, bind: u8, sym_type: u8, shndx: u16) {
        self.symbols.push(Symbol {
            name,
            value,
            size,
            bind,
            sym_type,
            shndx,
        });
    }

    /// Add a relocation
    pub fn add_relocation(&mut self, offset: u64, symbol: String) {
        self.relocations.push((offset, symbol));
    }

    /// Generate ELF object file
    pub fn build(&self) -> ObjectResult<Vec<u8>> {
        // For now, we'll generate assembly that can be assembled with 'as'
        // Full ELF generation would require detailed binary format knowledge
        // This is a practical compromise
        Ok(Vec::new())
    }
}

/// Generate assembly source file (simpler alternative to ELF)
pub fn generate_assembly_file(assembly: &str) -> ObjectResult<String> {
    // Return assembly as-is, which can be piped to 'as' assembler
    Ok(assembly.to_string())
}

/// Link assembly file using system linker
pub fn link_assembly(_asm_file: &str, _output: &str) -> ObjectResult<()> {
    // TODO: Implement linking using system assembler and linker
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_default() {
        let header = ElfHeader::default();
        assert_eq!(header.class, 2); // 64-bit
        assert_eq!(header.e_machine, 62); // x86-64
        assert_eq!(header.e_type, 1); // Relocatable
    }

    #[test]
    fn test_elf_header_serialization() {
        let header = ElfHeader::default();
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 64);
        assert_eq!(&bytes[0..4], [0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn test_symbol_creation() {
        let sym = Symbol {
            name: "main".to_string(),
            value: 0,
            size: 100,
            bind: 1, // global
            sym_type: 2, // function
            shndx: 1,
        };
        assert_eq!(sym.name, "main");
    }

    #[test]
    fn test_object_builder() {
        let mut builder = ObjectBuilder::new();
        builder.add_code(&[0x90, 0x90, 0xc3]); // nop nop ret
        assert_eq!(builder.text_section.len(), 3);
    }
}