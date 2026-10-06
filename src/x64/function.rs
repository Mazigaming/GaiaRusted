//! Code generation for one function.
//!
//! Every local has a home: a register picked by [`regalloc`](super::regalloc)
//! or a slot in the stack frame. Each IR statement is translated on its
//! own: it brings its operands into scratch registers (or uses them where
//! they are), computes, and puts the result in the destination's home. The
//! statement reads everything it needs before it writes, which is what
//! lets an operand that dies here share a register with the result.
//!
//! Register roles:
//!
//! * `rax`, `rcx`, `rdx`, `xmm0`, `xmm1` hold operands and results;
//! * `r10`, `r11` are used to compute addresses;
//! * every other general register, and `xmm8` to `xmm15`, can be a home.
//!
//! A register home holds the full 64-bit image of its value: an integer
//! narrower than that is kept sign- or zero-extended according to its type,
//! so it can be used in 64-bit arithmetic and comparisons as it is.

use super::abi::{classify, layout_call, ArgLoc, CallLayout, Class};
use super::reg::{Mem, Reg};
use super::regalloc::{self, Allocation, Register, CALLEE_SAVED};
use crate::ir::analysis::{BitSet, Liveness};
use crate::ir::layout::TagEncoding;
use crate::ir::opt::OptLevel;
use crate::ir::visit::LocalUse;
use crate::ir::*;
use crate::sema::context::Context;
use crate::sema::ty::Ty;
use crate::syntax::ast::BinOp;
use std::fmt::Write;

/// Where a local lives.
#[derive(Clone, Copy, PartialEq)]
enum Home {
    /// Zero-sized: nowhere.
    None,
    /// In the frame, starting this many bytes below `rbp`.
    Slot(i64),
    /// Elsewhere; the frame slot this many bytes below `rbp` holds its
    /// address. Used for by-memory parameters and for the return value
    /// when the caller provides the memory for it.
    Behind(i64),
    /// In a general register.
    Reg(Reg),
    /// A fat pointer in two general registers: data pointer, extra word.
    RegPair(Reg, Reg),
    /// In `xmm<n>`.
    Xmm(u8),
}

pub(super) struct FnEmitter<'e, 'c, 'a> {
    out: &'e mut String,
    tcx: &'c Context<'a>,
    program: &'e Program,
    func: &'e Function,
    func_index: usize,
    homes: Vec<Home>,
    frame_size: i64,
    signature: CallLayout,
    /// The callee-saved registers this function uses, each with the frame
    /// slot its previous value waits in.
    saved: Vec<(Reg, i64)>,
    /// The locals whose every use is visible: the ones liveness follows.
    whole: BitSet,
    /// For each block, which of those still have their value needed after it.
    live_out: Vec<BitSet>,
    /// How many labels inside statements have been handed out.
    local_labels: usize,
}

/// Blocks at or below this size are copied with a few moves instead of `rep movsb`.
const INLINE_COPY_LIMIT: u64 = 64;

impl<'e, 'c, 'a> FnEmitter<'e, 'c, 'a> {
    pub fn new(
        out: &'e mut String,
        tcx: &'c Context<'a>,
        program: &'e Program,
        func_index: usize,
        level: OptLevel,
    ) -> FnEmitter<'e, 'c, 'a> {
        let func = &program.functions[func_index];
        let param_tys: Vec<Ty> = func.args().map(|arg| func.locals[arg.0 as usize].ty.clone()).collect();
        let signature = layout_call(tcx, &param_tys, func.ret_ty());
        let allocation = match level {
            OptLevel::None => Allocation::none(func),
            OptLevel::Full if func.calls_returns_twice() => Allocation::none(func),
            OptLevel::Full => regalloc::allocate(func, tcx, &signature),
        };

        // Parameters and the return value exist whether or not the body
        // mentions them; any other local needs a home only if it is used.
        let mut used = vec![false; func.locals.len()];
        used[..=func.arg_count].fill(true);
        let mut note = |local: Local, _: LocalUse| used[local.0 as usize] = true;
        for block in &func.blocks {
            block.statements.iter().for_each(|statement| statement.each_local(&mut note));
            if let Some(terminator) = &block.terminator {
                terminator.each_local(&mut note);
            }
        }

        // Lay out the frame: an 8-byte-aligned slot for each local kept in memory.
        let mut frame_size = 0;
        let mut slot = |size: u64| {
            frame_size += size.div_ceil(8) as i64 * 8;
            frame_size
        };
        let mut homes = Vec::with_capacity(func.locals.len());
        for (index, local) in func.locals.iter().enumerate() {
            let is_param = (1..=func.arg_count).contains(&index);
            homes.push(match (allocation.registers[index], classify(tcx, &local.ty)) {
                _ if !used[index] => Home::None,
                (Some(Register::Int(reg)), _) => Home::Reg(reg),
                (Some(Register::Float(xmm)), _) => Home::Xmm(xmm),
                (Some(Register::Pair(data, extra)), _) => Home::RegPair(data, extra),
                (None, Class::Zst) => Home::None,
                (None, Class::Memory { .. }) if index == 0 || is_param => Home::Behind(slot(8)),
                (None, Class::Memory { size }) => Home::Slot(slot(size)),
                (None, Class::Pair) => Home::Slot(slot(16)),
                (None, Class::Int { .. } | Class::Float { .. }) => Home::Slot(slot(8)),
            });
        }
        let saved = CALLEE_SAVED.into_iter().filter(|&reg| allocation.uses(reg)).map(|reg| (reg, slot(8))).collect();
        // Keep `rsp` 16-byte aligned at every call.
        frame_size = (frame_size + 15) / 16 * 16;

        let whole = func.whole_locals();
        let live_out = Liveness::compute(func, &whole).live_out;
        FnEmitter { out, tcx, program, func, func_index, homes, frame_size, signature, saved, whole, live_out, local_labels: 0 }
    }

    pub fn emit(mut self) {
        let _ = writeln!(self.out, "\n# {}", self.func.name);
        let _ = writeln!(self.out, "{}:", self.func.symbol);
        self.ins("push rbp");
        self.ins("mov rbp, rsp");
        if self.frame_size > 0 {
            self.ins(format!("sub rsp, {}", self.frame_size));
        }
        for (reg, offset) in self.saved.clone() {
            self.ins(format!("mov {}, {reg}", self.slot(offset).sized(8)));
        }
        self.receive_params();

        for (index, block) in self.func.blocks.iter().enumerate() {
            let _ = writeln!(self.out, "{}:", self.label(BlockId(index as u32)));
            let terminator = block.terminator.as_ref().expect("blocks are terminated before code generation");
            match self.compare_feeding_branch(index, block) {
                Some((compare, then_block, else_block)) => {
                    let (_, before) = block.statements.split_last().expect("the compare is the last statement");
                    before.iter().for_each(|statement| self.statement(statement));
                    self.compare_and_branch(compare, then_block, else_block, index);
                }
                None => {
                    block.statements.iter().for_each(|statement| self.statement(statement));
                    self.terminator(terminator, index);
                }
            }
        }
    }

    fn ins(&mut self, instruction: impl AsRef<str>) {
        let _ = writeln!(self.out, "    {}", instruction.as_ref());
    }

    fn label(&self, block: BlockId) -> String {
        format!(".Lf{}_b{}", self.func_index, block.0)
    }

    /// A fresh label for control flow within one statement.
    fn local_label(&mut self) -> String {
        self.local_labels += 1;
        format!(".Lf{}_l{}", self.func_index, self.local_labels)
    }

    /// Put the double with these bits in `xmm<n>`.
    fn load_double_bits(&mut self, xmm: u8, bits: u64) {
        self.load_immediate(Reg::Rcx, bits as i64);
        self.ins(format!("movq xmm{xmm}, rcx"));
    }

    // -- types ------------------------------------------------------------------

    fn local_ty(&self, local: Local) -> &'e Ty {
        &self.func.locals[local.0 as usize].ty
    }

    fn class(&self, ty: &Ty) -> Class {
        classify(self.tcx, ty)
    }

    fn place_ty(&self, place: &Place) -> Ty {
        self.func.place_ty(self.tcx, place)
    }

    fn operand_ty(&self, operand: &Operand) -> Ty {
        self.func.operand_ty(self.tcx, operand)
    }

    // -- places -------------------------------------------------------------------

    fn slot(&self, offset: i64) -> Mem {
        Mem::at(Reg::Rbp, -offset)
    }

    /// The register `place` lives in, if it is a local kept in a general register.
    fn reg_home(&self, place: &Place) -> Option<Reg> {
        match self.homes[place.as_local()?.0 as usize] {
            Home::Reg(reg) => Some(reg),
            _ => None,
        }
    }

    /// The registers a fat-pointer operand lives in, if it is a local kept in them.
    fn pair_home(&self, operand: &Operand) -> Option<(Reg, Reg)> {
        let Operand::Copy(place) = operand else { return None };
        match self.homes[place.as_local()?.0 as usize] {
            Home::RegPair(data, extra) => Some((data, extra)),
            _ => None,
        }
    }

    /// The SSE register `place` lives in, if it is a local kept in one.
    fn xmm_home(&self, place: &Place) -> Option<u8> {
        match self.homes[place.as_local()?.0 as usize] {
            Home::Xmm(xmm) => Some(xmm),
            _ => None,
        }
    }

    /// Emit whatever is needed to address `place` and return the operand
    /// for it. May use `r10` and `r11`; the result is valid until they are
    /// used again. The place must be in memory: a local kept in a register
    /// has no address, though what it points to has.
    fn address(&mut self, place: &Place) -> Mem {
        let mut ty = self.local_ty(place.local).clone();
        let mut projections = place.projection.iter();
        let mut mem = match self.homes[place.local.0 as usize] {
            Home::Slot(offset) => self.slot(offset),
            Home::Behind(offset) => {
                self.ins(format!("mov r11, {}", self.slot(offset).sized(8)));
                Mem::at(Reg::R11, 0)
            }
            Home::Reg(pointer) | Home::RegPair(pointer, _) => {
                let Some(Projection::Deref) = projections.next() else {
                    unreachable!("a local kept in a register is only addressed through");
                };
                ty = ty.pointee().expect("deref of a non-pointer").clone();
                if let Home::RegPair(_, metadata) = self.homes[place.local.0 as usize] {
                    if self.has_dynamic_tail(&ty) {
                        self.ins(format!("mov r10, {metadata}"));
                    }
                }
                Mem::at(pointer, 0)
            }
            Home::Xmm(_) => unreachable!("a float is not an address"),
            // Never read or written; any address will do.
            Home::None => self.slot(0),
        };
        let mut variant = None;

        for projection in projections {
            match projection {
                Projection::Downcast(index) => {
                    variant = Some(*index);
                    continue;
                }
                Projection::Field(index) => {
                    if let Ty::Array(element, _) = &ty {
                        mem = mem.offset((*index as u64 * self.tcx.size_of(element)) as i64);
                        ty = (**element).clone();
                    } else if variant.is_none() && self.is_dynamic_tail(&ty, *index) {
                        mem = self.dynamic_tail_address(mem, &ty);
                        ty = self.tcx.field_types(&ty, None).swap_remove(*index);
                    } else {
                        mem = mem.offset(self.tcx.field_offset(&ty, variant, *index) as i64);
                        ty = self.tcx.field_types(&ty, variant).swap_remove(*index);
                    }
                }
                Projection::Deref => {
                    // For a fat pointer this loads its first word, the data
                    // pointer; the vtable a trait object at the end of the
                    // pointee may be laid out by goes to r10.
                    if self.has_dynamic_tail(ty.pointee().expect("deref of a non-pointer")) {
                        self.ins(format!("mov r10, {}", mem.offset(8).sized(8)));
                    }
                    self.ins(format!("mov r11, {}", mem.sized(8)));
                    mem = Mem::at(Reg::R11, 0);
                    ty = ty.pointee().expect("deref of a non-pointer").clone();
                }
                Projection::Index(index) => {
                    let element = match &ty {
                        Ty::Array(element, _) | Ty::Slice(element) => (**element).clone(),
                        other => unreachable!("cannot index `{}`", self.tcx.display(other)),
                    };
                    // One index register per operand: fold an earlier one into the base.
                    if mem.index.is_some() {
                        self.ins(format!("lea r11, {mem}"));
                        mem = Mem::at(Reg::R11, 0);
                    }
                    let size = self.tcx.size_of(&element);
                    let scalable = matches!(size, 1 | 2 | 4 | 8);
                    let index = match self.homes[index.0 as usize] {
                        Home::Reg(index) if scalable => index,
                        Home::Reg(index) => {
                            self.ins(format!("imul r10, {index}, {size}"));
                            Reg::R10
                        }
                        Home::Slot(offset) if scalable => {
                            self.ins(format!("mov r10, {}", self.slot(offset).sized(8)));
                            Reg::R10
                        }
                        Home::Slot(offset) => {
                            self.ins(format!("imul r10, {}, {size}", self.slot(offset).sized(8)));
                            Reg::R10
                        }
                        _ => unreachable!("an index is a `usize` local"),
                    };
                    mem.index = Some((index, if scalable { size } else { 1 }));
                    ty = element;
                }
            }
            variant = None;
        }
        mem
    }

    /// Does a value of this type end in a trait object, whose alignment
    /// decides where it starts?
    fn has_dynamic_tail(&self, ty: &Ty) -> bool {
        matches!(ty, Ty::Adt(..)) && self.tcx.is_unsized(ty) && self.tcx.unsized_align(ty).1
    }

    /// Is field `index` of `ty` the last one, holding a trait object?
    fn is_dynamic_tail(&self, ty: &Ty, index: usize) -> bool {
        self.has_dynamic_tail(ty) && index + 1 == self.tcx.field_types(ty, None).len()
    }

    /// The address of the last field of the struct at `base`, which holds a
    /// trait object (with its vtable in r10): after the other fields,
    /// rounded up to the alignment the vtable gives. The struct is aligned
    /// at least as much as that, so rounding the address up is the same.
    /// The vtable stays in r10 for a trait object further in. (The frame
    /// is addressed from rbp, so a push in between disturbs nothing.)
    fn dynamic_tail_address(&mut self, base: Mem, ty: &Ty) -> Mem {
        let tail = self.tcx.unsized_tail(ty);
        self.ins("push r10");
        self.ins("mov r10, qword ptr [r10 + 8]");
        if tail.static_align > 1 {
            let done = self.local_label();
            self.ins(format!("cmp r10, {}", tail.static_align));
            self.ins(format!("jae {done}"));
            self.ins(format!("mov r10, {}", tail.static_align));
            self.ins(format!("{done}:"));
        }
        self.ins(format!("lea r11, {base}"));
        self.ins(format!("lea r11, [r11 + r10 + {}]", tail.prefix_end as i64 - 1));
        self.ins("neg r10");
        self.ins("and r11, r10");
        self.ins("pop r10");
        Mem::at(Reg::R11, 0)
    }

    // -- loading and storing ---------------------------------------------------------

    /// The 64-bit register image of an integer constant of the given class:
    /// truncated to its size, then sign- or zero-extended.
    fn canonical(bits: u128, class: Class) -> i64 {
        let Class::Int { size, signed } = class else { unreachable!("integer constant of non-integer type") };
        let shift = 64 - size * 8;
        let truncated = (bits as u64) << shift;
        if signed {
            (truncated as i64) >> shift
        } else {
            (truncated >> shift) as i64
        }
    }

    fn load_immediate(&mut self, reg: Reg, value: i64) {
        if value == 0 {
            self.ins(format!("xor {0}, {0}", reg.name(4)));
        } else if i32::try_from(value).is_ok() {
            self.ins(format!("mov {reg}, {value}"));
        } else {
            self.ins(format!("movabs {reg}, {value}"));
        }
    }

    /// Load an integer-class value into `reg`, extended to 64 bits.
    fn load_int(&mut self, operand: &Operand, reg: Reg) {
        match operand {
            Operand::Const(Const::Int(bits, ty)) => {
                let value = Self::canonical(*bits, self.class(ty));
                self.load_immediate(reg, value);
            }
            Operand::Const(Const::DataAddr(data, _)) => {
                self.ins(format!("lea {reg}, [rip + {}]", self.program.data[data.0 as usize].symbol));
            }
            Operand::Const(Const::FuncAddr(func, _)) => {
                self.ins(format!("lea {reg}, [rip + {}]", self.program.functions[func.0 as usize].symbol));
            }
            Operand::Const(other) => unreachable!("{other:?} is not an integer-class value"),
            Operand::Copy(place) => match self.reg_home(place) {
                Some(home) if home == reg => {}
                Some(home) => self.ins(format!("mov {reg}, {home}")),
                None => {
                    let class = self.class(&self.place_ty(place));
                    let mem = self.address(place);
                    self.load_int_from(mem, class, reg);
                }
            },
        }
    }

    /// An integer constant that fits in an instruction's 32-bit immediate.
    fn immediate(&self, operand: &Operand) -> Option<i64> {
        let Operand::Const(Const::Int(bits, ty)) = operand else { return None };
        let value = Self::canonical(*bits, self.class(ty));
        i32::try_from(value).is_ok().then_some(value)
    }

    /// Is the operand available without touching memory: a constant, or a
    /// local kept in a register?
    fn is_simple(&self, operand: &Operand) -> bool {
        match operand {
            Operand::Const(_) => true,
            Operand::Copy(place) => self.reg_home(place).is_some(),
        }
    }

    /// Build a fat pointer from two simple operands straight in the pair of
    /// registers it lives in. Either half may currently hold the other's
    /// operand, so the order of the moves matters, and if both do, they swap.
    fn make_fat_in(&mut self, (data_home, extra_home): (Reg, Reg), data: &Operand, extra: &Operand) {
        let home_of = |operand: &Operand| match operand {
            Operand::Copy(place) => self.reg_home(place),
            Operand::Const(_) => None,
        };
        let (data_from, extra_from) = (home_of(data), home_of(extra));
        if data_from == Some(extra_home) && extra_from == Some(data_home) {
            self.ins(format!("xchg {data_home}, {extra_home}"));
        } else if extra_from == Some(data_home) {
            self.load_int(extra, extra_home);
            self.load_int(data, data_home);
        } else {
            self.load_int(data, data_home);
            self.load_int(extra, extra_home);
        }
    }

    /// A register holding an integer-class value: the operand's home if it
    /// has one, else `scratch` with the value loaded into it.
    fn int_reg(&mut self, operand: &Operand, scratch: Reg) -> Reg {
        if let Operand::Copy(place) = operand {
            if let Some(home) = self.reg_home(place) {
                return home;
            }
        }
        self.load_int(operand, scratch);
        scratch
    }

    /// An integer-class value as the second operand of a 64-bit
    /// instruction: an immediate if it is a constant that fits, else a
    /// register as for [`int_reg`](Self::int_reg).
    fn int_source(&mut self, operand: &Operand, scratch: Reg) -> String {
        if let Operand::Const(Const::Int(bits, ty)) = operand {
            let value = Self::canonical(*bits, self.class(ty));
            if i32::try_from(value).is_ok() {
                return value.to_string();
            }
        }
        self.int_reg(operand, scratch).to_string()
    }

    /// Move `from` to `to`, extending the low `size` bytes to the full
    /// register the way a value of this class is kept.
    fn extend(&mut self, to: Reg, from: Reg, class: Class) {
        let Class::Int { size, signed } = class else { unreachable!("{class:?} is not integer-class") };
        let instruction = match (size, signed) {
            (8, _) if to == from => return,
            (8, _) => format!("mov {to}, {from}"),
            (4, false) => format!("mov {}, {}", to.name(4), from.name(4)),
            (4, true) => format!("movsxd {to}, {}", from.name(4)),
            (_, false) => format!("movzx {}, {}", to.name(4), from.name(size)),
            (_, true) => format!("movsx {to}, {}", from.name(size)),
        };
        self.ins(instruction);
    }

    fn load_int_from(&mut self, mem: Mem, class: Class, reg: Reg) {
        let Class::Int { size, signed } = class else { unreachable!("{class:?} is not integer-class") };
        let instruction = match (size, signed) {
            (8, _) => format!("mov {reg}, {}", mem.sized(8)),
            // Writing a 32-bit register clears the upper half.
            (4, false) => format!("mov {}, {}", reg.name(4), mem.sized(4)),
            (4, true) => format!("movsxd {reg}, {}", mem.sized(4)),
            (_, false) => format!("movzx {}, {}", reg.name(4), mem.sized(size)),
            (_, true) => format!("movsx {reg}, {}", mem.sized(size)),
        };
        self.ins(instruction);
    }

    /// Load a float into `xmm<n>`.
    fn load_float(&mut self, operand: &Operand, xmm: u8) {
        match operand {
            Operand::Const(Const::Float(value, ty)) => match self.class(ty) {
                Class::Float { size: 4 } => {
                    self.ins(format!("mov eax, {}", (*value as f32).to_bits()));
                    self.ins(format!("movd xmm{xmm}, eax"));
                }
                _ => {
                    self.ins(format!("movabs rax, {}", value.to_bits() as i64));
                    self.ins(format!("movq xmm{xmm}, rax"));
                }
            },
            Operand::Copy(place) => match self.xmm_home(place) {
                Some(home) if home == xmm => {}
                Some(home) => self.ins(format!("movaps xmm{xmm}, xmm{home}")),
                None => {
                    let Class::Float { size } = self.class(&self.place_ty(place)) else {
                        unreachable!("float load of a non-float place");
                    };
                    let mem = self.address(place);
                    self.ins(format!("{} xmm{xmm}, {}", float_mov(size), mem.sized(size)));
                }
            },
            Operand::Const(other) => unreachable!("{other:?} is not a float"),
        }
    }

    /// The SSE register holding a float: the operand's home if it has one,
    /// else `xmm<scratch>` with the value loaded into it.
    fn float_reg(&mut self, operand: &Operand, scratch: u8) -> u8 {
        if let Operand::Copy(place) = operand {
            if let Some(home) = self.xmm_home(place) {
                return home;
            }
        }
        self.load_float(operand, scratch);
        scratch
    }

    /// Load a two-word pointer into the given registers.
    fn load_pair(&mut self, operand: &Operand, first: Reg, second: Reg) {
        if let Some((data, extra)) = self.pair_home(operand) {
            // A register pair is never the scratch registers this loads into.
            if first != data {
                self.ins(format!("mov {first}, {data}"));
            }
            if second != extra {
                self.ins(format!("mov {second}, {extra}"));
            }
            return;
        }
        match operand {
            Operand::Const(Const::Str(data, len)) => {
                self.ins(format!("lea {first}, [rip + {}]", self.program.data[data.0 as usize].symbol));
                self.load_immediate(second, *len as i64);
            }
            Operand::Copy(place) => {
                let mem = self.address(place);
                self.ins(format!("mov {first}, {}", mem.sized(8)));
                self.ins(format!("mov {second}, {}", mem.offset(8).sized(8)));
            }
            // A 128-bit integer: the low word, then the high one.
            Operand::Const(Const::Int(bits, _)) => {
                self.load_immediate(first, *bits as u64 as i64);
                self.load_immediate(second, (*bits >> 64) as u64 as i64);
            }
            Operand::Const(other) => unreachable!("{other:?} does not fill two registers"),
        }
    }

    /// Store the value sitting in the result registers (`rax`, `rax:rdx` or
    /// `xmm0`, by class) into `place`.
    fn store_result(&mut self, place: &Place, class: Class) {
        if class == Class::Zst {
            return;
        }
        if let Some(home) = self.reg_home(place) {
            return self.extend(home, Reg::Rax, class);
        }
        if let Some(home) = self.xmm_home(place) {
            return self.ins(format!("movaps xmm{home}, xmm0"));
        }
        if let Some((data, extra)) = self.pair_home(&Operand::Copy(place.clone())) {
            self.ins(format!("mov {data}, rax"));
            self.ins(format!("mov {extra}, rdx"));
            return;
        }
        let mem = self.address(place);
        match class {
            Class::Int { size, .. } => self.ins(format!("mov {}, {}", mem.sized(size), Reg::Rax.name(size))),
            Class::Float { size } => self.ins(format!("{} {}, xmm0", float_mov(size), mem.sized(size))),
            Class::Pair => {
                self.ins(format!("mov {}, rax", mem.sized(8)));
                self.ins(format!("mov {}, rdx", mem.offset(8).sized(8)));
            }
            Class::Zst | Class::Memory { .. } => unreachable!("{class:?} values are not held in registers"),
        }
    }

    /// Copy a two-word value without passing it through `rax:rdx`, when
    /// one side is a register pair. Returns whether it did.
    fn copy_pair(&mut self, source: &Operand, dest: &Place) -> bool {
        let into = self.pair_home(&Operand::Copy(dest.clone()));
        match (into, self.pair_home(source)) {
            (Some((data, extra)), Some((from_data, from_extra))) => {
                if data == from_extra && extra == from_data {
                    self.ins(format!("xchg {data}, {extra}"));
                } else if data == from_extra {
                    // Writing the data word first would destroy the other.
                    self.ins(format!("mov {extra}, {from_extra}"));
                    self.ins(format!("mov {data}, {from_data}"));
                } else {
                    if data != from_data {
                        self.ins(format!("mov {data}, {from_data}"));
                    }
                    if extra != from_extra {
                        self.ins(format!("mov {extra}, {from_extra}"));
                    }
                }
                true
            }
            (Some((data, extra)), None) => {
                let Operand::Copy(place) = source else { return false };
                let mem = self.address(place);
                let uses = |reg: Reg| mem.base == reg || mem.index.is_some_and(|(index, _)| index == reg);
                match (uses(data), uses(extra)) {
                    (false, _) => {
                        self.ins(format!("mov {data}, {}", mem.sized(8)));
                        self.ins(format!("mov {extra}, {}", mem.offset(8).sized(8)));
                    }
                    (true, false) => {
                        self.ins(format!("mov {extra}, {}", mem.offset(8).sized(8)));
                        self.ins(format!("mov {data}, {}", mem.sized(8)));
                    }
                    (true, true) => return false,
                }
                true
            }
            (None, Some((from_data, from_extra))) => {
                let mem = self.address(dest);
                self.ins(format!("mov {}, {from_data}", mem.sized(8)));
                self.ins(format!("mov {}, {from_extra}", mem.offset(8).sized(8)));
                true
            }
            (None, None) => false,
        }
    }

    /// Load a value of any register class into the result registers.
    fn load_result(&mut self, operand: &Operand, class: Class) {
        match class {
            Class::Int { .. } => self.load_int(operand, Reg::Rax),
            Class::Float { .. } => self.load_float(operand, 0),
            Class::Pair => self.load_pair(operand, Reg::Rax, Reg::Rdx),
            Class::Zst => {}
            Class::Memory { .. } => unreachable!("memory-class values are copied, not loaded"),
        }
    }

    /// Copy `size` bytes from the memory `source` occupies to `dest`.
    fn copy_memory(&mut self, source: &Operand, dest: &Place, size: u64) {
        let Operand::Copy(source) = source else { unreachable!("memory-class constants do not exist") };
        let from = self.address(source);
        self.ins(format!("lea rax, {from}"));
        let to = self.address(dest);
        self.ins(format!("lea rdx, {to}"));

        if size > INLINE_COPY_LIMIT {
            // The string instruction insists on `rsi`, `rdi` and `rcx`;
            // the first two may be somebody's home.
            let is_home = |reg: &Reg| {
                self.homes.iter().any(|home| match *home {
                    Home::Reg(home) => home == *reg,
                    Home::RegPair(data, extra) => data == *reg || extra == *reg,
                    _ => false,
                })
            };
            let homes: Vec<Reg> = [Reg::Rsi, Reg::Rdi].into_iter().filter(is_home).collect();
            homes.iter().for_each(|reg| self.ins(format!("push {reg}")));
            self.ins("mov rsi, rax");
            self.ins("mov rdi, rdx");
            self.ins(format!("mov rcx, {size}"));
            self.ins("rep movsb");
            homes.iter().rev().for_each(|reg| self.ins(format!("pop {reg}")));
            return;
        }
        let mut offset = 0;
        for chunk in [8, 4, 2, 1] {
            while size - offset >= chunk {
                let (from, to) = (Mem::at(Reg::Rax, offset as i64), Mem::at(Reg::Rdx, offset as i64));
                self.ins(format!("mov {}, {}", Reg::Rcx.name(chunk), from.sized(chunk)));
                self.ins(format!("mov {}, {}", to.sized(chunk), Reg::Rcx.name(chunk)));
                offset += chunk;
            }
        }
    }

    // -- statements -----------------------------------------------------------------------

    fn statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Assign(place, rvalue) => self.assign(place, rvalue),
            Statement::SetDiscriminant(place, variant) => self.set_discriminant(place, *variant),
            Statement::Call { dest, callee, args } => self.call(dest, callee, args),
        }
    }

    /// Record in an enum which variant it holds.
    fn set_discriminant(&mut self, place: &Place, variant: u32) {
        let ty = self.place_ty(place);
        let Ty::Adt(adt, _) = &ty else { unreachable!("only enums have a discriminant") };
        match self.tcx.tag_encoding(&ty) {
            TagEncoding::Direct(tag) => {
                let value = self.tcx.discriminant(*adt, variant);
                let mem = self.address(place);
                self.ins(format!("mov {}, {}", mem.sized(tag.size()), value as i64));
            }
            // The dataful variant is told apart by its fields alone.
            TagEncoding::Niche { dataful, .. } if variant == dataful => {}
            TagEncoding::Niche { first, niche, value, .. } => {
                let stored = value.wrapping_add((variant - first) as u128) as u64;
                let mem = self.address(place).offset(niche.offset as i64);
                if niche.size == 8 && i32::try_from(stored as i64).is_err() {
                    self.ins(format!("movabs rax, {}", stored as i64));
                    self.ins(format!("mov {}, rax", mem.sized(8)));
                } else {
                    self.ins(format!("mov {}, {}", mem.sized(niche.size), sign_extended(stored, niche.size)));
                }
            }
        }
    }

    /// Load the discriminant of the enum at `place` into rax.
    fn load_discriminant(&mut self, place: &Place) {
        let ty = self.place_ty(place);
        let (dataful, first, count, niche, value) = match self.tcx.tag_encoding(&ty) {
            TagEncoding::Direct(tag) => {
                let mem = self.address(place);
                self.load_int_from(mem, Class::Int { size: tag.size(), signed: tag.is_signed() }, Reg::Rax);
                return;
            }
            TagEncoding::Niche { dataful, first, count, niche, value } => (dataful, first, count, niche, value),
        };
        let mem = self.address(place).offset(niche.offset as i64);
        let value = value as u64;
        let fits = niche.size < 8 || i32::try_from(value as i64).is_ok();
        if count == 1 && fits {
            // Just one variant without data: the niche holds its value or not.
            self.ins(format!("cmp {}, {}", mem.sized(niche.size), sign_extended(value, niche.size)));
            self.ins(format!("mov eax, {dataful}"));
            self.ins(format!("mov edx, {first}"));
            self.ins("cmove eax, edx".to_string());
            return;
        }
        // How far past the first reserved value the niche holds, wrapping
        // around at the scalar's size; below `count` it names a variant.
        self.load_int_from(mem, Class::Int { size: niche.size, signed: false }, Reg::Rax);
        if niche.size == 8 {
            self.ins(format!("movabs rcx, {}", value as i64));
            self.ins("sub rax, rcx".to_string());
        } else {
            self.ins(format!("sub eax, {}", value as u32 as i32));
            if niche.size < 4 {
                self.ins(format!("movzx eax, {}", Reg::Rax.name(niche.size)));
            }
        }
        self.ins(format!("lea rdx, [rax + {first}]"));
        self.ins(format!("cmp rax, {}", count - 1));
        self.ins(format!("mov eax, {dataful}"));
        self.ins("cmovbe eax, edx".to_string());
    }

    fn assign(&mut self, place: &Place, rvalue: &Rvalue) {
        let dest_class = self.class(&self.place_ty(place));
        if let Some(home) = self.reg_home(place) {
            if self.assign_in_register(home, dest_class, rvalue) {
                return;
            }
        }
        if let Some(home) = self.xmm_home(place) {
            if self.assign_in_xmm_register(home, rvalue) {
                return;
            }
        }
        match rvalue {
            Rvalue::Use(operand) => match dest_class {
                Class::Memory { size } => self.copy_memory(operand, place, size),
                Class::Float { .. } if self.xmm_home(place).is_some() => {
                    let home = self.xmm_home(place).expect("checked by the guard");
                    self.load_float(operand, home);
                }
                // A constant goes to memory as an immediate.
                Class::Int { size, .. } if self.reg_home(place).is_none() && self.immediate(operand).is_some() => {
                    let value = self.immediate(operand).expect("checked by the guard");
                    let mem = self.address(place);
                    self.ins(format!("mov {}, {value}", mem.sized(size)));
                }
                Class::Pair if self.copy_pair(operand, place) => {}
                class => {
                    self.load_result(operand, class);
                    self.store_result(place, class);
                }
            },
            Rvalue::Binary(op, lhs, rhs) => {
                let operand_class = self.class(&self.operand_ty(lhs));
                match operand_class {
                    Class::Float { size } => self.float_binary(*op, lhs, rhs, size),
                    Class::Int { signed, .. } => self.int_binary(*op, lhs, rhs, signed),
                    Class::Pair => {
                        let signed = matches!(self.operand_ty(lhs), Ty::Int(int) if int.is_signed());
                        self.wide_binary(*op, lhs, rhs, signed)
                    }
                    other => unreachable!("binary operator on {other:?} operands"),
                }
                self.store_result(place, dest_class);
            }
            Rvalue::Unary(op, operand) => {
                let class = self.class(&self.operand_ty(operand));
                self.load_result(operand, class);
                match (op, class) {
                    (UnaryOp::Neg, Class::Int { .. }) => self.ins("neg rax"),
                    (UnaryOp::Not, Class::Int { .. }) if self.operand_ty(operand) == Ty::Bool => self.ins("xor eax, 1"),
                    (UnaryOp::Not, Class::Int { .. }) => self.ins("not rax"),
                    // Negating a float flips its sign bit.
                    (UnaryOp::Neg, Class::Float { size: 8 }) => {
                        self.ins("movq rax, xmm0");
                        self.ins("btc rax, 63");
                        self.ins("movq xmm0, rax");
                    }
                    (UnaryOp::Neg, Class::Float { .. }) => {
                        self.ins("movd eax, xmm0");
                        self.ins("btc eax, 31");
                        self.ins("movd xmm0, eax");
                    }
                    (UnaryOp::Sqrt, Class::Float { size }) => self.ins(format!("sqrt{0} xmm0, xmm0", float_suffix(size))),
                    // A 128-bit `-x`: negate the low word, then the high one
                    // and the borrow out of the low.
                    (UnaryOp::Neg, Class::Pair) => {
                        self.ins("neg rax");
                        self.ins("adc rdx, 0");
                        self.ins("neg rdx");
                    }
                    (UnaryOp::Not, Class::Pair) => {
                        self.ins("not rax");
                        self.ins("not rdx");
                    }
                    (op, class) => unreachable!("{op:?} on a {class:?} operand"),
                }
                self.store_result(place, dest_class);
            }
            Rvalue::Cast(operand, from, to) => {
                self.cast(operand, self.class(from), self.class(to));
                self.store_result(place, dest_class);
            }
            Rvalue::AddrOf(target) => {
                let mem = self.address(target);
                self.ins(format!("lea rax, {mem}"));
                self.store_result(place, dest_class);
            }
            Rvalue::Discriminant(of) => {
                self.load_discriminant(of);
                self.store_result(place, dest_class);
            }
            Rvalue::MakeFat(data, extra) => match self.pair_home(&Operand::Copy(place.clone())) {
                Some(home) if self.is_simple(data) && self.is_simple(extra) => self.make_fat_in(home, data, extra),
                _ => {
                    self.load_int(data, Reg::Rax);
                    self.load_int(extra, Reg::Rdx);
                    self.store_result(place, Class::Pair);
                }
            },
            Rvalue::FatData(pointer) | Rvalue::FatExtra(pointer) => {
                let is_data = matches!(rvalue, Rvalue::FatData(_));
                match (self.pair_home(pointer), pointer) {
                    (Some((data, extra)), _) => {
                        let half = if is_data { data } else { extra };
                        self.ins(format!("mov rax, {half}"));
                    }
                    // Only the word wanted is read.
                    (None, Operand::Copy(source)) => {
                        let mem = self.address(source);
                        let mem = if is_data { mem } else { mem.offset(8) };
                        self.ins(format!("mov rax, {}", mem.sized(8)));
                    }
                    (None, constant) if is_data => self.load_pair(constant, Reg::Rax, Reg::Rdx),
                    (None, constant) => self.load_pair(constant, Reg::Rdx, Reg::Rax),
                }
                self.store_result(place, dest_class);
            }
        }
    }

    /// Compute an integer-class rvalue right in `home`, the register of
    /// the local it is assigned to. Returns `false`, having emitted
    /// nothing, for rvalues that take the general route through `rax`.
    ///
    /// `home` may also be the register of an operand that dies here, so it
    /// is written only once every other operand has been read.
    fn assign_in_register(&mut self, home: Reg, class: Class, rvalue: &Rvalue) -> bool {
        match rvalue {
            // Source and destination have the same type, so the value is
            // already in the form its home keeps it in.
            Rvalue::Use(operand) => self.load_int(operand, home),
            Rvalue::Cast(operand, from, _) if matches!(self.class(from), Class::Int { .. }) => {
                let source = self.int_reg(operand, Reg::Rax);
                self.extend(home, source, class);
            }
            Rvalue::FatData(pointer) | Rvalue::FatExtra(pointer) if self.pair_home(pointer).is_some() => {
                let (data, extra) = self.pair_home(pointer).expect("checked by the guard");
                let half = if matches!(rvalue, Rvalue::FatData(_)) { data } else { extra };
                if half != home {
                    self.ins(format!("mov {home}, {half}"));
                }
            }
            Rvalue::Binary(op, lhs, rhs) if !op.is_comparison() => {
                let Class::Int { .. } = self.class(&self.operand_ty(lhs)) else { return false };
                let instruction = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    BinOp::Mul => "imul",
                    BinOp::BitAnd => "and",
                    BinOp::BitOr => "or",
                    BinOp::BitXor => "xor",
                    BinOp::Shl | BinOp::Shr if matches!(rhs, Operand::Const(_)) => {
                        let Operand::Const(Const::Int(count, _)) = rhs else { return false };
                        let Class::Int { signed, .. } = self.class(&self.operand_ty(lhs)) else { return false };
                        let shift = match (op, signed) {
                            (BinOp::Shl, _) => "shl",
                            (_, true) => "sar",
                            (_, false) => "shr",
                        };
                        self.load_int(lhs, home);
                        self.ins(format!("{shift} {home}, {}", count & 63));
                        self.extend(home, home, class);
                        return true;
                    }
                    _ => return false,
                };
                let is_home = |operand: &Operand| matches!(operand, Operand::Copy(place) if self.reg_home(place) == Some(home));
                // `x = y - x` cannot be done in `x`'s register without a copy.
                let (lhs, rhs) = match (is_home(lhs), is_home(rhs)) {
                    (false, true) if *op == BinOp::Sub => return false,
                    (false, true) => (rhs, lhs),
                    _ => (lhs, rhs),
                };
                let source = self.int_source(rhs, Reg::Rcx);
                self.load_int(lhs, home);
                self.ins(format!("{instruction} {home}, {source}"));
                self.extend(home, home, class);
            }
            Rvalue::Binary(op, lhs, rhs) => {
                let Class::Int { signed, .. } = self.class(&self.operand_ty(lhs)) else { return false };
                let condition = self.compare(*op, lhs, rhs, signed);
                self.ins(format!("set{condition} {}", home.name(1)));
                self.extend(home, home, class);
            }
            _ => return false,
        }
        true
    }

    /// Compute a float rvalue right in `xmm<home>`, the register of the
    /// local it is assigned to; the counterpart of
    /// [`assign_in_register`](Self::assign_in_register).
    fn assign_in_xmm_register(&mut self, home: u8, rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Binary(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div), lhs, rhs) => {
                let Class::Float { size } = self.class(&self.operand_ty(lhs)) else { return false };
                let is_home = |operand: &Operand| matches!(operand, Operand::Copy(place) if self.xmm_home(place) == Some(home));
                let (lhs, rhs) = match (is_home(lhs), is_home(rhs)) {
                    (false, true) if matches!(op, BinOp::Sub | BinOp::Div) => return false,
                    (false, true) => (rhs, lhs),
                    _ => (lhs, rhs),
                };
                let instruction = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    BinOp::Mul => "mul",
                    _ => "div",
                };
                self.load_float(lhs, home);
                let source = self.float_source(rhs, 1, size);
                self.ins(format!("{instruction}{} xmm{home}, {source}", float_suffix(size)));
            }
            Rvalue::Unary(UnaryOp::Sqrt, operand) => {
                let Class::Float { size } = self.class(&self.operand_ty(operand)) else { return false };
                let source = self.float_source(operand, 1, size);
                self.ins(format!("sqrt{} xmm{home}, {source}", float_suffix(size)));
            }
            _ => return false,
        }
        true
    }

    /// A float as the second operand of an SSE instruction: its register,
    /// its memory, or `xmm<scratch>` holding a constant.
    fn float_source(&mut self, operand: &Operand, scratch: u8, size: u64) -> String {
        match operand {
            Operand::Copy(place) => match self.xmm_home(place) {
                Some(home) => format!("xmm{home}"),
                None => {
                    let mem = self.address(place);
                    mem.sized(size)
                }
            },
            Operand::Const(_) => {
                self.load_float(operand, scratch);
                format!("xmm{scratch}")
            }
        }
    }

    /// Integer `lhs op rhs`, result in `rax` (comparisons leave 0 or 1).
    fn int_binary(&mut self, op: BinOp, lhs: &Operand, rhs: &Operand, signed: bool) {
        if op.is_comparison() {
            let condition = self.compare(op, lhs, rhs, signed);
            self.ins(format!("set{condition} al"));
            self.ins("movzx eax, al");
            return;
        }
        self.load_int(lhs, Reg::Rax);
        let instruction = match op {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "imul",
            BinOp::BitAnd => "and",
            BinOp::BitOr => "or",
            BinOp::BitXor => "xor",
            BinOp::Shl | BinOp::Shr => {
                let shift = match (op, signed) {
                    (BinOp::Shl, _) => "shl",
                    (_, true) => "sar",
                    (_, false) => "shr",
                };
                // The count is an immediate or sits in `cl`.
                match rhs {
                    Operand::Const(Const::Int(count, _)) => self.ins(format!("{shift} rax, {}", count & 63)),
                    _ => {
                        self.load_int(rhs, Reg::Rcx);
                        self.ins(format!("{shift} rax, cl"));
                    }
                }
                return;
            }
            BinOp::Div | BinOp::Rem => {
                if let Operand::Const(Const::Int(divisor, ty)) = rhs {
                    let divisor = Self::canonical(*divisor, self.class(ty)) as u64;
                    if !signed && divisor > 2 {
                        return self.divide_by_constant(op, divisor);
                    }
                }
                // The dividend is `rdx:rax`; the remainder ends up in `rdx`.
                let divisor = self.int_reg(rhs, Reg::Rcx);
                if signed {
                    self.ins("cqo");
                    self.ins(format!("idiv {divisor}"));
                } else {
                    self.ins("xor edx, edx");
                    self.ins(format!("div {divisor}"));
                }
                if op == BinOp::Rem {
                    self.ins("mov rax, rdx");
                }
                return;
            }
            _ => unreachable!("`{}` is not an arithmetic operator", op.symbol()),
        };
        let source = self.int_source(rhs, Reg::Rcx);
        self.ins(format!("{instruction} rax, {source}"));
    }

    /// `lhs op rhs` on 128-bit integers, kept as `rax:rdx` (low word
    /// first): arithmetic leaves the result there, a comparison 0 or 1 in
    /// `rax`. Division is a call, made before this point.
    fn wide_binary(&mut self, op: BinOp, lhs: &Operand, rhs: &Operand, signed: bool) {
        if op.is_comparison() {
            // `a < b` is the sign (signed) or borrow (unsigned) of the
            // 128-bit `a - b`; the other orderings swap the operands or
            // negate the answer.
            let (first, second, condition) = match (op, signed) {
                (BinOp::Eq | BinOp::Ne, _) => {
                    self.load_pair(lhs, Reg::Rax, Reg::Rdx);
                    self.load_pair(rhs, Reg::Rcx, Reg::R11);
                    self.ins("xor rax, rcx");
                    self.ins("xor rdx, r11");
                    self.ins("or rax, rdx");
                    let condition = if op == BinOp::Eq { "e" } else { "ne" };
                    self.ins(format!("set{condition} al"));
                    self.ins("movzx eax, al");
                    return;
                }
                (BinOp::Lt, true) => (lhs, rhs, "l"),
                (BinOp::Ge, true) => (lhs, rhs, "ge"),
                (BinOp::Gt, true) => (rhs, lhs, "l"),
                (BinOp::Le, true) => (rhs, lhs, "ge"),
                (BinOp::Lt, false) => (lhs, rhs, "b"),
                (BinOp::Ge, false) => (lhs, rhs, "ae"),
                (BinOp::Gt, false) => (rhs, lhs, "b"),
                (BinOp::Le, false) => (rhs, lhs, "ae"),
                _ => unreachable!("`{}` is not a comparison", op.symbol()),
            };
            self.load_pair(first, Reg::Rax, Reg::Rdx);
            self.load_pair(second, Reg::Rcx, Reg::R11);
            self.ins("cmp rax, rcx");
            self.ins("sbb rdx, r11");
            self.ins(format!("set{condition} al"));
            self.ins("movzx eax, al");
            return;
        }
        match op {
            BinOp::Add | BinOp::Sub | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                let (low, high) = match op {
                    BinOp::Add => ("add", "adc"),
                    BinOp::Sub => ("sub", "sbb"),
                    BinOp::BitAnd => ("and", "and"),
                    BinOp::BitOr => ("or", "or"),
                    _ => ("xor", "xor"),
                };
                self.load_pair(lhs, Reg::Rax, Reg::Rdx);
                self.load_pair(rhs, Reg::Rcx, Reg::R11);
                self.ins(format!("{low} rax, rcx"));
                self.ins(format!("{high} rdx, r11"));
            }
            BinOp::Mul => {
                // (a_hi 2^64 + a_lo)(b_hi 2^64 + b_lo) mod 2^128 is
                // a_lo b_lo plus the two cross products in the high word.
                self.load_pair(lhs, Reg::R10, Reg::R11);
                self.load_pair(rhs, Reg::Rax, Reg::Rdx);
                self.ins("imul r11, rax");
                self.ins("imul rdx, r10");
                self.ins("add r11, rdx");
                self.ins("mul r10");
                self.ins("add rdx, r11");
            }
            BinOp::Shl | BinOp::Shr => {
                self.load_pair(lhs, Reg::Rax, Reg::Rdx);
                match self.class(&self.operand_ty(rhs)) {
                    Class::Pair => self.load_pair(rhs, Reg::Rcx, Reg::R11),
                    _ => self.load_int(rhs, Reg::Rcx),
                }
                // Shift across the two words by the count modulo 64, then
                // move a whole word over if the count is 64 or more.
                let done = self.local_label();
                match (op, signed) {
                    (BinOp::Shl, _) => {
                        self.ins("shld rdx, rax, cl");
                        self.ins("shl rax, cl");
                        self.ins("test cl, 64");
                        self.ins(format!("je {done}"));
                        self.ins("mov rdx, rax");
                        self.ins("xor eax, eax");
                    }
                    (_, false) => {
                        self.ins("shrd rax, rdx, cl");
                        self.ins("shr rdx, cl");
                        self.ins("test cl, 64");
                        self.ins(format!("je {done}"));
                        self.ins("mov rax, rdx");
                        self.ins("xor edx, edx");
                    }
                    (_, true) => {
                        self.ins("shrd rax, rdx, cl");
                        self.ins("sar rdx, cl");
                        self.ins("test cl, 64");
                        self.ins(format!("je {done}"));
                        self.ins("mov rax, rdx");
                        self.ins("sar rdx, 63");
                    }
                }
                self.ins_label(&done);
            }
            _ => unreachable!("`{}` on 128-bit integers is a call", op.symbol()),
        }
    }

    /// Unsigned `rax / divisor` or `rax % divisor` into `rax`, without a
    /// division instruction.
    ///
    /// Dividing by `d` is multiplying by `1/d`. With `l = ceil(log2 d)`
    /// and the 64-bit multiplier `m = floor(2^64 * (2^l - d) / d) + 1`,
    /// the quotient of any 64-bit `n` is
    ///
    /// ```text
    /// t = (m * n) >> 64
    /// q = (t + ((n - t) >> 1)) >> (l - 1)
    /// ```
    ///
    /// (Granlund and Montgomery, "Division by Invariant Integers using
    /// Multiplication", figure 4.1.)
    fn divide_by_constant(&mut self, op: BinOp, divisor: u64) {
        let l = 64 - (divisor - 1).leading_zeros();
        let multiplier = ((1u128 << 64) * ((1u128 << l) - divisor as u128) / divisor as u128 + 1) as u64;
        self.ins("mov rcx, rax");
        self.load_immediate(Reg::Rax, multiplier as i64);
        self.ins("mul rcx");
        self.ins("mov rax, rcx");
        self.ins("sub rax, rdx");
        self.ins("shr rax, 1");
        self.ins("add rax, rdx");
        self.ins(format!("shr rax, {}", l - 1));
        if op == BinOp::Rem {
            // n - q * d
            match i32::try_from(divisor) {
                Ok(divisor) => self.ins(format!("imul rax, rax, {divisor}")),
                Err(_) => {
                    self.load_immediate(Reg::Rdx, divisor as i64);
                    self.ins("imul rax, rdx");
                }
            }
            self.ins("sub rcx, rax");
            self.ins("mov rax, rcx");
        }
    }

    /// Compare two integers, leaving the answer in the flags. Returns the
    /// condition code under which `lhs op rhs` holds.
    fn compare(&mut self, op: BinOp, lhs: &Operand, rhs: &Operand, signed: bool) -> &'static str {
        let left = self.int_reg(lhs, Reg::Rax);
        let right = self.int_source(rhs, Reg::Rcx);
        // Comparing with zero sets the flags the same way as testing.
        if right == "0" {
            self.ins(format!("test {left}, {left}"));
        } else {
            self.ins(format!("cmp {left}, {right}"));
        }
        match (op, signed) {
            (BinOp::Eq, _) => "e",
            (BinOp::Ne, _) => "ne",
            (BinOp::Lt, true) => "l",
            (BinOp::Le, true) => "le",
            (BinOp::Gt, true) => "g",
            (BinOp::Ge, true) => "ge",
            (BinOp::Lt, false) => "b",
            (BinOp::Le, false) => "be",
            (BinOp::Gt, false) => "a",
            (BinOp::Ge, false) => "ae",
            _ => unreachable!("`{}` is not a comparison", op.symbol()),
        }
    }

    /// If the block ends by branching on an integer comparison it has just
    /// made and whose outcome is not needed afterwards, the comparison and
    /// where the branch goes.
    fn compare_feeding_branch(&self, index: usize, block: &'e Block) -> Option<(Comparison<'e>, BlockId, BlockId)> {
        let Some(Terminator::Branch { cond: Operand::Copy(cond), then_block, else_block }) = &block.terminator else {
            return None;
        };
        let Some(Statement::Assign(result, Rvalue::Binary(op, lhs, rhs))) = block.statements.last() else {
            return None;
        };
        let flag = cond.as_local().filter(|_| result == cond)?;
        let Class::Int { signed, .. } = self.class(&self.operand_ty(lhs)) else { return None };
        let flag = flag.0 as usize;
        let needed_later = !self.whole.contains(flag) || self.live_out[index].contains(flag);
        let fusible = op.is_comparison() && !needed_later;
        fusible.then_some((Comparison { op: *op, lhs, rhs, signed }, *then_block, *else_block))
    }

    /// Branch on a comparison straight from the flags, without turning the
    /// outcome into a `bool` first.
    fn compare_and_branch(&mut self, compare: Comparison, then_block: BlockId, else_block: BlockId, block_index: usize) {
        let holds = self.compare(compare.op, compare.lhs, compare.rhs, compare.signed);
        if then_block.0 as usize == block_index + 1 {
            self.ins(format!("j{} {}", negated(holds), self.label(else_block)));
        } else {
            self.ins(format!("j{holds} {}", self.label(then_block)));
            if else_block.0 as usize != block_index + 1 {
                self.ins(format!("jmp {}", self.label(else_block)));
            }
        }
    }

    /// Float `lhs op rhs`: arithmetic leaves the result in `xmm0`,
    /// comparisons leave 0 or 1 in `rax`.
    fn float_binary(&mut self, op: BinOp, lhs: &Operand, rhs: &Operand, size: u64) {
        self.load_float(lhs, 0);
        let suffix = float_suffix(size);
        let arithmetic = match op {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::Div => "div",
            BinOp::Rem => {
                self.load_float(rhs, 1);
                self.ins(if size == 8 { "call fmod" } else { "call fmodf" });
                return;
            }
            _ => "",
        };
        let rhs = self.float_reg(rhs, 1);
        if !arithmetic.is_empty() {
            self.ins(format!("{arithmetic}{suffix} xmm0, xmm{rhs}"));
            return;
        }

        // `ucomis` sets the flags as for an unsigned compare, and the parity
        // flag when either operand is NaN. Every ordered comparison with NaN
        // must be false; phrasing `<` as a swapped `>` gets that for free.
        let compare = format!("ucomi{suffix}");
        match op {
            BinOp::Lt | BinOp::Le => {
                self.ins(format!("{compare} xmm{rhs}, xmm0"));
                self.ins(if op == BinOp::Lt { "seta al" } else { "setae al" });
            }
            BinOp::Gt | BinOp::Ge => {
                self.ins(format!("{compare} xmm0, xmm{rhs}"));
                self.ins(if op == BinOp::Gt { "seta al" } else { "setae al" });
            }
            BinOp::Eq => {
                self.ins(format!("{compare} xmm0, xmm{rhs}"));
                self.ins("sete al");
                self.ins("setnp cl");
                self.ins("and al, cl");
            }
            BinOp::Ne => {
                self.ins(format!("{compare} xmm0, xmm{rhs}"));
                self.ins("setne al");
                self.ins("setp cl");
                self.ins("or al, cl");
            }
            other => unreachable!("`{}` is not defined for floats", other.symbol()),
        }
        self.ins("movzx eax, al");
    }

    /// Convert `operand` from one primitive class to another, leaving the
    /// result in the result registers of the target class.
    fn cast(&mut self, operand: &Operand, from: Class, to: Class) {
        match (from, to) {
            // Loading already extends by the source type; storing truncates
            // to the target type. Nothing else to do.
            (Class::Int { .. }, Class::Int { .. }) => self.load_int(operand, Reg::Rax),
            (Class::Int { size: 8, signed: false }, Class::Float { size }) => {
                // The instruction converts signed numbers. A `u64` with the
                // top bit set is halved first (keeping the lowest bit, so it
                // still rounds right) and the result doubled.
                self.load_int(operand, Reg::Rax);
                let (large, done) = (self.local_label(), self.local_label());
                let suffix = float_suffix(size);
                self.ins("test rax, rax");
                self.ins(format!("js {large}"));
                self.ins(format!("cvtsi2{suffix} xmm0, rax"));
                self.ins(format!("jmp {done}"));
                self.ins_label(&large);
                self.ins("mov rcx, rax");
                self.ins("shr rcx, 1");
                self.ins("and eax, 1");
                self.ins("or rcx, rax");
                self.ins(format!("cvtsi2{suffix} xmm0, rcx"));
                self.ins(format!("add{suffix} xmm0, xmm0"));
                self.ins_label(&done);
            }
            (Class::Int { .. }, Class::Float { size }) => {
                self.load_int(operand, Reg::Rax);
                self.ins(format!("cvtsi2{} xmm0, rax", float_suffix(size)));
            }
            (Class::Float { size }, Class::Int { size: to_size, signed }) => {
                self.load_float(operand, 0);
                if size == 4 {
                    self.ins("cvtss2sd xmm0, xmm0");
                }
                self.float_to_int(to_size, signed);
            }
            (Class::Float { size: from }, Class::Float { size: to }) => {
                self.load_float(operand, 0);
                match (from, to) {
                    (8, 4) => self.ins("cvtsd2ss xmm0, xmm0"),
                    (4, 8) => self.ins("cvtss2sd xmm0, xmm0"),
                    _ => {}
                }
            }
            // A fat pointer cast to a thin one keeps the data pointer, and a
            // 128-bit integer narrowed keeps its low word.
            (Class::Pair, Class::Int { .. } | Class::Pair) => self.load_pair(operand, Reg::Rax, Reg::Rdx),
            // Widened to 128 bits: the high word repeats the sign, or is 0.
            (Class::Int { signed, .. }, Class::Pair) => {
                self.load_int(operand, Reg::Rax);
                if signed {
                    self.ins("mov rdx, rax");
                    self.ins("sar rdx, 63");
                } else {
                    self.ins("xor edx, edx");
                }
            }
            (from, to) => unreachable!("no cast from {from:?} to {to:?}"),
        }
    }

    /// Convert the double in `xmm0` to an integer of the given size in
    /// `rax`, as Rust's `as` does: towards zero, saturating at the type's
    /// limits, with NaN becoming 0.
    fn float_to_int(&mut self, size: u64, signed: bool) {
        let done = self.local_label();
        self.ins("xor eax, eax");
        match (size, signed) {
            (8, true) => {
                // Too negative already gives `i64::MIN`; too large must be clamped.
                let too_large = self.local_label();
                self.ins("ucomisd xmm0, xmm0");
                self.ins(format!("jp {done}"));
                self.load_double_bits(1, 0x43E0_0000_0000_0000);
                self.ins("ucomisd xmm0, xmm1");
                self.ins(format!("jae {too_large}"));
                self.ins("cvttsd2si rax, xmm0");
                self.ins(format!("jmp {done}"));
                self.ins_label(&too_large);
                self.load_immediate(Reg::Rax, i64::MAX);
            }
            (8, false) => {
                // Above 2^63 the conversion is done on `x - 2^63` and the top
                // bit put back.
                let (too_large, upper_half) = (self.local_label(), self.local_label());
                self.ins("xorpd xmm1, xmm1");
                self.ins("ucomisd xmm0, xmm1");
                self.ins(format!("jbe {done}"));
                self.load_double_bits(1, 0x43F0_0000_0000_0000);
                self.ins("ucomisd xmm0, xmm1");
                self.ins(format!("jae {too_large}"));
                self.load_double_bits(1, 0x43E0_0000_0000_0000);
                self.ins("ucomisd xmm0, xmm1");
                self.ins(format!("jae {upper_half}"));
                self.ins("cvttsd2si rax, xmm0");
                self.ins(format!("jmp {done}"));
                self.ins_label(&upper_half);
                self.ins("subsd xmm0, xmm1");
                self.ins("cvttsd2si rax, xmm0");
                self.ins("btc rax, 63");
                self.ins(format!("jmp {done}"));
                self.ins_label(&too_large);
                self.ins("mov rax, -1");
            }
            (_, signed) => {
                // Every limit of a narrower type is a double, so clamping
                // first keeps the conversion in range.
                let bits = size * 8;
                let (low, high) = if signed {
                    (-((1i64 << (bits - 1)) as f64), ((1i64 << (bits - 1)) - 1) as f64)
                } else {
                    (0.0, ((1u64 << bits) - 1) as f64)
                };
                self.ins("ucomisd xmm0, xmm0");
                self.ins(format!("jp {done}"));
                self.load_double_bits(1, high.to_bits());
                self.ins("minsd xmm0, xmm1");
                self.load_double_bits(1, low.to_bits());
                self.ins("maxsd xmm0, xmm1");
                self.ins("cvttsd2si rax, xmm0");
            }
        }
        self.ins_label(&done);
    }

    fn ins_label(&mut self, label: &str) {
        let _ = writeln!(self.out, "{label}:");
    }

    // -- calls ---------------------------------------------------------------------------

    fn call(&mut self, dest: &Place, callee: &Callee, args: &[Operand]) {
        let is_virtual = matches!(callee, Callee::Virtual { .. });
        let mut arg_tys: Vec<Ty> = args.iter().map(|arg| self.operand_ty(arg)).collect();
        if is_virtual {
            // The method receives the object's data pointer only.
            arg_tys[0] = Ty::Ptr(Box::new(Ty::UNIT), crate::sema::ty::Mutability::Not);
        }
        let ret_ty = self.place_ty(dest);
        let layout = layout_call(self.tcx, &arg_tys, &ret_ty);

        // Stack arguments, keeping `rsp` 16-byte aligned at the call.
        let stack_bytes = (layout.stack_words + layout.stack_words % 2) as i64 * 8;
        if stack_bytes > 0 {
            self.ins(format!("sub rsp, {stack_bytes}"));
        }
        for (arg, (class, loc)) in args.iter().zip(&layout.args) {
            let ArgLoc::Stack(word) = loc else { continue };
            let slot = Mem::at(Reg::Rsp, *word as i64 * 8);
            match class {
                Class::Int { .. } => {
                    self.load_int(arg, Reg::Rax);
                    self.ins(format!("mov {}, rax", slot.sized(8)));
                }
                Class::Float { size } => {
                    self.load_float(arg, 0);
                    self.ins(format!("{} {}, xmm0", float_mov(*size), slot.sized(*size)));
                }
                Class::Pair => {
                    self.load_pair(arg, Reg::Rax, Reg::Rdx);
                    self.ins(format!("mov {}, rax", slot.sized(8)));
                    self.ins(format!("mov {}, rdx", slot.offset(8).sized(8)));
                }
                Class::Memory { .. } => {
                    self.load_address(arg, Reg::Rax);
                    self.ins(format!("mov {}, rax", slot.sized(8)));
                }
                Class::Zst => {}
            }
        }

        // The hidden pointer for a result returned in memory.
        if layout.returns_in_memory() {
            let mem = self.address(dest);
            self.ins(format!("lea rdi, {mem}"));
        }

        // Register arguments. Loading one only touches its own registers
        // and the address scratch registers, so the order does not matter.
        let mut vtable_receiver = None;
        for (index, (arg, (class, loc))) in args.iter().zip(&layout.args).enumerate() {
            match (class, loc) {
                (_, ArgLoc::Regs(regs)) if is_virtual && index == 0 => vtable_receiver = Some((arg, regs[0])),
                (Class::Int { .. }, ArgLoc::Regs(regs)) => self.load_int(arg, regs[0]),
                (Class::Pair, ArgLoc::Regs(regs)) => self.load_pair(arg, regs[0], regs[1]),
                (Class::Memory { .. }, ArgLoc::Regs(regs)) => self.load_address(arg, regs[0]),
                (Class::Float { .. }, ArgLoc::Xmm(xmm)) => self.load_float(arg, *xmm),
                (_, ArgLoc::Stack(_) | ArgLoc::None) => {}
                (class, loc) => unreachable!("{class:?} argument in {loc:?}"),
            }
        }

        match callee {
            Callee::Direct(func) => self.ins(format!("call {}", self.program.functions[func.0 as usize].symbol)),
            Callee::Extern { symbol, variadic } => {
                if *variadic {
                    self.ins(format!("mov eax, {}", layout.xmm_used));
                }
                self.ins(format!("call {symbol}"));
            }
            Callee::Indirect(pointer) => {
                self.load_int(pointer, Reg::R10);
                self.ins("call r10");
            }
            Callee::Virtual { index } => {
                let (receiver, data_reg) = vtable_receiver.expect("a virtual call has a receiver");
                self.load_pair(receiver, data_reg, Reg::Rax);
                let entry = Mem::at(Reg::Rax, (index * 8) as i64);
                self.ins(format!("call {}", entry.sized(8)));
            }
        }
        if stack_bytes > 0 {
            self.ins(format!("add rsp, {stack_bytes}"));
        }
        if !layout.returns_in_memory() {
            self.store_result(dest, layout.ret);
        }
    }

    /// The address of a memory-class argument.
    fn load_address(&mut self, operand: &Operand, reg: Reg) {
        let Operand::Copy(place) = operand else { unreachable!("memory-class constants do not exist") };
        let mem = self.address(place);
        self.ins(format!("lea {reg}, {mem}"));
    }

    /// Move the incoming arguments from where the caller put them to the
    /// parameters' homes.
    fn receive_params(&mut self) {
        if self.signature.returns_in_memory() {
            let Home::Behind(offset) = self.homes[0] else { unreachable!("memory-class return without a pointer slot") };
            self.ins(format!("mov {}, rdi", self.slot(offset).sized(8)));
        }
        // Stack arguments sit above the saved `rbp` and the return address.
        let incoming = |word: usize| Mem::at(Reg::Rbp, 16 + word as i64 * 8);

        // Parameters bound for memory are stored at once. Those bound for
        // registers wait: a home may be the register another argument is
        // still sitting in.
        let mut between_registers = Vec::new();
        let mut from_stack = Vec::new();
        let mut in_registers = Vec::new();
        for (index, (class, loc)) in self.signature.args.clone().into_iter().enumerate() {
            let home = match self.homes[index + 1] {
                Home::Slot(offset) | Home::Behind(offset) => self.slot(offset),
                Home::None => continue,
                Home::Reg(home) => {
                    match loc {
                        ArgLoc::Regs(regs) => between_registers.push((regs[0], home)),
                        ArgLoc::Stack(word) => from_stack.push((incoming(word), class, home)),
                        loc => unreachable!("{class:?} parameter in {loc:?}"),
                    }
                    in_registers.push((home, class));
                    continue;
                }
                Home::RegPair(data, extra) => {
                    let word = Class::Int { size: 8, signed: false };
                    match loc {
                        ArgLoc::Regs(regs) => between_registers.extend([(regs[0], data), (regs[1], extra)]),
                        ArgLoc::Stack(at) => from_stack.extend([(incoming(at), word, data), (incoming(at + 1), word, extra)]),
                        loc => unreachable!("{class:?} parameter in {loc:?}"),
                    }
                    continue;
                }
                // Float homes are never argument registers.
                Home::Xmm(home) => {
                    let Class::Float { size } = class else { unreachable!("{class:?} parameter in an SSE register") };
                    match loc {
                        ArgLoc::Xmm(xmm) => self.ins(format!("movaps xmm{home}, xmm{xmm}")),
                        ArgLoc::Stack(word) => {
                            self.ins(format!("{} xmm{home}, {}", float_mov(size), incoming(word).sized(size)))
                        }
                        loc => unreachable!("{class:?} parameter in {loc:?}"),
                    }
                    continue;
                }
            };
            match (class, loc) {
                (Class::Int { size, .. }, ArgLoc::Regs(regs)) => {
                    self.ins(format!("mov {}, {}", home.sized(size), regs[0].name(size)));
                }
                (Class::Memory { .. }, ArgLoc::Regs(regs)) => self.ins(format!("mov {}, {}", home.sized(8), regs[0])),
                (Class::Pair, ArgLoc::Regs(regs)) => {
                    self.ins(format!("mov {}, {}", home.sized(8), regs[0]));
                    self.ins(format!("mov {}, {}", home.offset(8).sized(8), regs[1]));
                }
                (Class::Float { size }, ArgLoc::Xmm(xmm)) => {
                    self.ins(format!("{} {}, xmm{xmm}", float_mov(size), home.sized(size)));
                }
                (Class::Pair, ArgLoc::Stack(word)) => {
                    for half in 0..2 {
                        self.ins(format!("mov rax, {}", incoming(word + half).sized(8)));
                        self.ins(format!("mov {}, rax", home.offset(half as i64 * 8).sized(8)));
                    }
                }
                (Class::Float { size }, ArgLoc::Stack(word)) => {
                    self.ins(format!("{} xmm0, {}", float_mov(size), incoming(word).sized(size)));
                    self.ins(format!("{} {}, xmm0", float_mov(size), home.sized(size)));
                }
                (Class::Int { size, .. }, ArgLoc::Stack(word)) => {
                    self.ins(format!("mov rax, {}", incoming(word).sized(8)));
                    self.ins(format!("mov {}, {}", home.sized(size), Reg::Rax.name(size)));
                }
                (Class::Memory { .. }, ArgLoc::Stack(word)) => {
                    self.ins(format!("mov rax, {}", incoming(word).sized(8)));
                    self.ins(format!("mov {}, rax", home.sized(8)));
                }
                (class, loc) => unreachable!("{class:?} parameter in {loc:?}"),
            }
        }

        self.move_together(between_registers);
        for (mem, class, home) in from_stack {
            self.load_int_from(mem, class, home);
        }
        // A caller need not have extended a narrow argument.
        for (home, class) in in_registers {
            if !matches!(class, Class::Int { size: 8, .. }) {
                self.extend(home, home, class);
            }
        }
    }

    /// Carry out a set of register-to-register moves as if all happened at
    /// the same instant: no move sees the result of another.
    fn move_together(&mut self, mut moves: Vec<(Reg, Reg)>) {
        moves.retain(|(from, to)| from != to);
        while !moves.is_empty() {
            // A target that no pending move still reads can be written now.
            let free = moves.iter().position(|(_, to)| !moves.iter().any(|(from, _)| from == to));
            match free {
                Some(index) => {
                    let (from, to) = moves.remove(index);
                    self.ins(format!("mov {to}, {from}"));
                }
                // Only cycles are left. Swapping completes one move of a
                // cycle; whoever was going to read its target now finds
                // that value in its source.
                None => {
                    let (from, to) = moves.remove(0);
                    self.ins(format!("xchg {to}, {from}"));
                    for (other_from, _) in &mut moves {
                        if *other_from == to {
                            *other_from = from;
                        }
                    }
                    moves.retain(|(from, to)| from != to);
                }
            }
        }
    }

    // -- terminators -------------------------------------------------------------------------

    fn terminator(&mut self, terminator: &Terminator, block_index: usize) {
        let is_next = |target: &BlockId| target.0 as usize == block_index + 1;
        match terminator {
            Terminator::Goto(target) => {
                if !is_next(target) {
                    self.ins(format!("jmp {}", self.label(*target)));
                }
            }
            Terminator::Branch { cond, then_block, else_block } => {
                let cond = self.int_reg(cond, Reg::Rax).name(4);
                self.ins(format!("test {cond}, {cond}"));
                if is_next(then_block) {
                    self.ins(format!("je {}", self.label(*else_block)));
                } else {
                    self.ins(format!("jne {}", self.label(*then_block)));
                    if !is_next(else_block) {
                        self.ins(format!("jmp {}", self.label(*else_block)));
                    }
                }
            }
            Terminator::Switch { value, arms, otherwise } => {
                let class = self.class(&self.operand_ty(value));
                let value = self.int_reg(value, Reg::Rax);
                for (constant, target) in arms {
                    let constant = Self::canonical(*constant, class);
                    if i32::try_from(constant).is_ok() {
                        self.ins(format!("cmp {value}, {constant}"));
                    } else {
                        self.load_immediate(Reg::Rcx, constant);
                        self.ins(format!("cmp {value}, rcx"));
                    }
                    self.ins(format!("je {}", self.label(*target)));
                }
                if !is_next(otherwise) {
                    self.ins(format!("jmp {}", self.label(*otherwise)));
                }
            }
            Terminator::Return => {
                let result = Operand::Copy(Place::local(RETURN_LOCAL));
                match self.signature.ret {
                    Class::Memory { .. } => {
                        // By convention the callee hands the result pointer back.
                        let Home::Behind(offset) = self.homes[0] else { unreachable!() };
                        self.ins(format!("mov rax, {}", self.slot(offset).sized(8)));
                    }
                    class => self.load_result(&result, class),
                }
                for (reg, offset) in self.saved.clone() {
                    self.ins(format!("mov {reg}, {}", self.slot(offset).sized(8)));
                }
                self.ins("leave");
                self.ins("ret");
            }
            Terminator::Unreachable => self.ins("ud2"),
        }
    }
}

/// An integer comparison whose outcome a branch consumes.
struct Comparison<'e> {
    op: BinOp,
    lhs: &'e Operand,
    rhs: &'e Operand,
    signed: bool,
}

/// The condition code that holds exactly when `condition` does not.
/// The low `size` bytes of `value` read as a signed number, as an
/// immediate operand of that size is written.
fn sign_extended(value: u64, size: u64) -> i64 {
    let unused = 64 - 8 * size;
    ((value << unused) as i64) >> unused
}

fn negated(condition: &str) -> &'static str {
    match condition {
        "e" => "ne",
        "ne" => "e",
        "l" => "ge",
        "le" => "g",
        "g" => "le",
        "ge" => "l",
        "b" => "ae",
        "be" => "a",
        "a" => "be",
        "ae" => "b",
        other => unreachable!("unknown condition code `{other}`"),
    }
}

fn float_suffix(size: u64) -> &'static str {
    if size == 4 {
        "ss"
    } else {
        "sd"
    }
}

fn float_mov(size: u64) -> &'static str {
    if size == 4 {
        "movss"
    } else {
        "movsd"
    }
}
