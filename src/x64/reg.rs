//! Registers and memory operands, and how they are spelled in assembly.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Rbp,
    Rsp,
}

impl Reg {
    /// The register's name when used as an operand of `size` bytes.
    pub fn name(self, size: u64) -> &'static str {
        let names: [&str; 4] = match self {
            Reg::Rax => ["al", "ax", "eax", "rax"],
            Reg::Rbx => ["bl", "bx", "ebx", "rbx"],
            Reg::Rcx => ["cl", "cx", "ecx", "rcx"],
            Reg::Rdx => ["dl", "dx", "edx", "rdx"],
            Reg::Rsi => ["sil", "si", "esi", "rsi"],
            Reg::Rdi => ["dil", "di", "edi", "rdi"],
            Reg::R8 => ["r8b", "r8w", "r8d", "r8"],
            Reg::R9 => ["r9b", "r9w", "r9d", "r9"],
            Reg::R10 => ["r10b", "r10w", "r10d", "r10"],
            Reg::R11 => ["r11b", "r11w", "r11d", "r11"],
            Reg::R12 => ["r12b", "r12w", "r12d", "r12"],
            Reg::R13 => ["r13b", "r13w", "r13d", "r13"],
            Reg::R14 => ["r14b", "r14w", "r14d", "r14"],
            Reg::R15 => ["r15b", "r15w", "r15d", "r15"],
            Reg::Rbp => ["bpl", "bp", "ebp", "rbp"],
            Reg::Rsp => ["spl", "sp", "esp", "rsp"],
        };
        match size {
            1 => names[0],
            2 => names[1],
            4 => names[2],
            8 => names[3],
            other => unreachable!("no {other}-byte register"),
        }
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name(8))
    }
}

/// A memory operand `[base + index * scale + disp]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mem {
    pub base: Reg,
    /// A register and what it is multiplied by: 1, 2, 4 or 8.
    pub index: Option<(Reg, u64)>,
    pub disp: i64,
}

impl Mem {
    pub fn at(base: Reg, disp: i64) -> Mem {
        Mem { base, index: None, disp }
    }

    pub fn offset(self, by: i64) -> Mem {
        Mem { disp: self.disp + by, ..self }
    }

    /// The operand with its size spelled out: `dword ptr [rbp - 12]`.
    pub fn sized(self, size: u64) -> String {
        let width = match size {
            1 => "byte",
            2 => "word",
            4 => "dword",
            8 => "qword",
            other => unreachable!("no {other}-byte memory operand"),
        };
        format!("{width} ptr {self}")
    }
}

impl fmt::Display for Mem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}", self.base)?;
        match self.index {
            Some((index, 1)) => write!(f, " + {index}")?,
            Some((index, scale)) => write!(f, " + {index} * {scale}")?,
            None => {}
        }
        match self.disp {
            0 => write!(f, "]"),
            disp if disp < 0 => write!(f, " - {}]", -disp),
            disp => write!(f, " + {disp}]"),
        }
    }
}
