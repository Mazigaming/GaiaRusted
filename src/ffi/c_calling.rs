//! # C Calling Convention Implementation
//!
//! Implements the C calling convention (System V AMD64 ABI) for x86-64.
//!
//! ## Features:
//! - Parameter passing in registers (rdi, rsi, rdx, rcx, r8, r9)
//! - Stack alignment to 16 bytes
//! - Return value in rax (or xmm0 for floats)
//! - Variadic function support

use std::fmt;
use super::c_types::CType;
use super::mod.rs::FfiFunctionSignature;

/// System V AMD64 ABI register allocation for function parameters
/// Integer/pointer arguments: rdi, rsi, rdx, rcx, r8, r9
/// Float arguments: xmm0-xmm7
/// Additional args go on stack (right to left)
pub struct CCallingConvention;

impl CCallingConvention {
    /// Generate function prologue for C function
    pub fn generate_prologue(func_name: &str) -> String {
        let mut code = String::new();
        code.push_str(&format!("# Prologue for C function: {}\n", func_name));
        code.push_str("push rbp\n");
        code.push_str("mov rbp, rsp\n");
        // Align stack to 16 bytes
        code.push_str("and rsp, -16\n");
        code
    }
    
    /// Generate function epilogue for C function
    pub fn generate_epilogue(func_name: &str) -> String {
        let mut code = String::new();
        code.push_str(&format!("# Epilogue for C function: {}\n", func_name));
        code.push_str("mov rsp, rbp\n");
        code.push_str("pop rbp\n");
        code.push_str("ret\n");
        code
    }
    
    /// Generate code to pass arguments according to C calling convention
    pub fn generate_argument_passing(
        args: &[(String, CType)],
        is_variadic: bool,
    ) -> String {
        let mut code = String::new();
        let mut int_reg_idx = 0;
        let mut float_reg_idx = 0;
        let mut stack_offset = 0;
        
        let int_regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
        let float_regs = ["xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7"];
        
        for (i, (arg_name, arg_type)) in args.iter().enumerate() {
            if CCallingConvention::is_float_type(arg_type) {
                if float_reg_idx < 8 {
                    // Pass in XMM register
                    code.push_str(&format!(
                        "movsd {}, [{}]\n",
                        float_regs[float_reg_idx], arg_name
                    ));
                    float_reg_idx += 1;
                } else {
                    // Pass on stack
                    code.push_str(&format!(
                        "mov qword [rsp + {}], [{}]\n",
                        stack_offset, arg_name
                    ));
                    stack_offset += 8;
                }
            } else {
                if int_reg_idx < 6 {
                    // Pass in integer register
                    code.push_str(&format!(
                        "mov {}, [{}]\n",
                        int_regs[int_reg_idx], arg_name
                    ));
                    int_reg_idx += 1;
                } else {
                    // Pass on stack
                    code.push_str(&format!(
                        "mov qword [rsp + {}], [{}]\n",
                        stack_offset, arg_name
                    ));
                    stack_offset += 8;
                }
            }
            
            // For variadic functions, set rax to number of floating point args
            if is_variadic && i == args.len() - 1 {
                code.push_str(&format!("mov rax, {}\n", float_reg_idx));
            }
        }
        
        code
    }
    
    /// Generate return value handling
    pub fn generate_return_value(return_type: &CType) -> String {
        let mut code = String::new();
        
        match return_type {
            CType::CVoid => {
                // No return value
            }
            CType::CFloat | CType::CDouble => {
                // Return in xmm0
                code.push_str("movsd xmm0, [result]\n");
            }
            _ => {
                // Return in rax
                code.push_str("mov rax, [result]\n");
            }
        }
        
        code
    }
    
    /// Check if a C type is a floating point type
    fn is_float_type(c_type: &CType) -> bool {
        matches!(c_type, CType::CFloat | CType::CDouble)
    }
    
    /// Generate function call instruction for C function
    pub fn generate_call(func_name: &str, args: &[(String, CType)]) -> String {
        let mut code = String::new();
        
        // Pass arguments
        code.push_str(&CCallingConvention::generate_argument_passing(args, false));
        
        // Align stack before call
        code.push_str("and rsp, -16\n");
        
        // Call function
        code.push_str(&format!("call {}\n", func_name));
        
        code
    }
    
    /// Generate variadic function call (like printf)
    pub fn generate_variadic_call(
        func_name: &str,
        args: &[(String, CType)],
    ) -> String {
        let mut code = String::new();
        
        // Pass arguments (including format string)
        code.push_str(&CCallingConvention::generate_argument_passing(args, true));
        
        // For variadic calls, al = number of XMM registers used
        let float_count = args.iter()
            .filter(|(_, t)| CCallingConvention::is_float_type(t))
            .count();
        code.push_str(&format!("mov al, {}\n", float_count));
        
        // Align stack and call
        code.push_str("and rsp, -16\n");
        code.push_str(&format!("call {}\n", func_name));
        
        code
    }
}

/// Generate complete C function wrapper
pub fn generate_c_wrapper(
    sig: &FfiFunctionSignature,
    body: &str,
) -> String {
    let mut code = String::new();
    
    // Function label (global for C linkage)
    code.push_str(&format!(".global {}\n", sig.name));
    code.push_str(&format!("{}:\n", sig.name));
    
    // Prologue
    code.push_str(&CCallingConvention::generate_prologue(&sig.name));
    
    // Function body
    code.push_str(body);
    
    // Epilogue
    code.push_str(&CCallingConvention::generate_epilogue(&sig.name));
    
    code
}

/// Generate C function declaration for extern block
pub fn generate_extern_decl(sig: &FfiFunctionSignature) -> String {
    let mut decl = String::new();
    
    // Return type
    decl.push_str(&format!("{} ", sig.return_type));
    
    // Function name
    decl.push_str(&sig.name);
    
    // Parameters
    decl.push_str("(");
    for (i, (name, c_type)) in sig.params.iter().enumerate() {
        if i > 0 { decl.push_str(", "); }
        decl.push_str(&format!("{} {}", c_type, name));
    }
    
    if sig.is_variadic {
        if !sig.params.is_empty() { decl.push_str(", "); }
        decl.push_str("...");
    }
    
    decl.push_str(")");
    
    // ABI
    decl.push_str(&format!(" /* ABI: {} */", sig.abi));
    
    decl
}

impl fmt::Display for CCallingConvention {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "System V AMD64 ABI (x86-64)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::c_types::CType;
    use crate::ffi::mod.rs::FfiFunctionSignature;
    use crate::ffi::mod.rs::ABI;
    
    #[test]
    fn test_generate_prologue() {
        let prologue = CCallingConvention::generate_prologue("test_func");
        assert!(prologue.contains("push rbp"));
        assert!(prologue.contains("mov rbp, rsp"));
        assert!(prologue.contains("and rsp, -16"));
    }
    
    #[test]
    fn test_generate_epilogue() {
        let epilogue = CCallingConvention::generate_epilogue("test_func");
        assert!(epilogue.contains("mov rsp, rbp"));
        assert!(epilogue.contains("pop rbp"));
        assert!(epilogue.contains("ret"));
    }
    
    #[test]
    fn test_generate_call() {
        let args = vec![
            ("a".to_string(), CType::CInt),
            ("b".to_string(), CType::CInt),
        ];
        let call = CCallingConvention::generate_call("add", &args);
        assert!(call.contains("mov rdi, [a]"));
        assert!(call.contains("mov rsi, [b]"));
        assert!(call.contains("call add"));
    }
    
    #[test]
    fn test_generate_variadic_call() {
        let args = vec![
            ("fmt".to_string(), CType::CPointer(Box::new(CType::CChar))),
            ("val".to_string(), CType::CInt),
        ];
        let call = CCallingConvention::generate_variadic_call("printf", &args);
        assert!(call.contains("call printf"));
        assert!(call.contains("mov al,"));
    }
    
    #[test]
    fn test_generate_c_wrapper() {
        let sig = FfiFunctionSignature::new(
            "my_func".to_string(),
            ABI::C,
            vec![("x".to_string(), CType::CInt)],
            CType::CInt,
            false,
        );
        let wrapper = generate_c_wrapper(&sig, "  # function body\n");
        assert!(wrapper.contains(".global my_func"));
        assert!(wrapper.contains("my_func:"));
        assert!(wrapper.contains("push rbp"));
    }
    
    #[test]
    fn test_is_float_type() {
        assert!(CCallingConvention::is_float_type(&CType::CFloat));
        assert!(CCallingConvention::is_float_type(&CType::CDouble));
        assert!(!CCallingConvention::is_float_type(&CType::CInt));
    }
}