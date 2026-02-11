//! Parse REPL input commands

/// Function definition result
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub return_type: String,
    pub body: String,
}

/// Parse a function definition
/// Example: fn add(a: i32, b: i32) -> i32 { a + b }
pub fn parse_function_def(input: &str) -> Result<FunctionDef, String> {
    let input = input.trim();
    
    if !input.starts_with("fn ") {
        return Err("function definition must start with 'fn'".to_string());
    }

    // Find function name
    let after_fn = &input[3..];
    let name_end = after_fn
        .find('(')
        .ok_or("missing '(' in function definition")?;
    let name = after_fn[..name_end].trim().to_string();

    if name.is_empty() {
        return Err("function name cannot be empty".to_string());
    }

    // Find parameters section
    let paren_start = after_fn.find('(').ok_or("missing '('")?;
    let paren_end = after_fn.find(')').ok_or("missing ')'")?;
    let params_str = &after_fn[paren_start + 1..paren_end].trim();
    
    // Parse parameters (simplified - just extract names)
    let params = if params_str.is_empty() {
        vec![]
    } else {
        params_str
            .split(',')
            .map(|p| {
                let parts: Vec<_> = p.trim().split(':').collect();
                if parts.len() == 2 {
                    (parts[0].trim().to_string(), parts[1].trim().to_string())
                } else {
                    ("".to_string(), "".to_string())
                }
            })
            .filter(|(n, t)| !n.is_empty() && !t.is_empty())
            .collect()
    };

    // Find return type
    let after_paren = &after_fn[paren_end + 1..].trim();
    let return_type = if let Some(arrow_pos) = after_paren.find("->") {
        let after_arrow = &after_paren[arrow_pos + 2..].trim();
        if let Some(brace_pos) = after_arrow.find('{') {
            after_arrow[..brace_pos].trim().to_string()
        } else {
            "()".to_string()
        }
    } else {
        "()".to_string()
    };

    // Find body
    let body = if let Some(brace_start) = after_paren.find('{') {
        if let Some(brace_end) = after_paren.rfind('}') {
            after_paren[brace_start + 1..brace_end].trim().to_string()
        } else {
            return Err("missing closing brace '}'".to_string());
        }
    } else {
        return Err("missing opening brace '{'".to_string());
    };

    Ok(FunctionDef {
        name,
        params,
        return_type,
        body,
    })
}

/// Parse a let binding
/// Example: let x: i32 = 5;
pub fn parse_let_binding(input: &str) -> Result<(String, String, String), String> {
    let input = input.trim();
    
    if !input.starts_with("let ") {
        return Err("variable definition must start with 'let'".to_string());
    }

    let after_let = &input[4..];
    
    // Find the colon (type annotation)
    let colon_pos = after_let.find(':')
        .ok_or("missing ':' in let binding")?;
    
    let name = after_let[..colon_pos].trim().to_string();
    
    if name.is_empty() {
        return Err("variable name cannot be empty".to_string());
    }

    // Find the equals sign
    let after_colon = &after_let[colon_pos + 1..];
    let eq_pos = after_colon.find('=')
        .ok_or("missing '=' in let binding")?;
    
    let var_type = after_colon[..eq_pos].trim().to_string();
    
    // Get the value
    let after_eq = &after_colon[eq_pos + 1..].trim();
    let value = if after_eq.ends_with(';') {
        &after_eq[..after_eq.len() - 1]
    } else {
        after_eq
    }.trim().to_string();

    Ok((name, var_type, value))
}

/// Parse a let mut binding
/// Example: let mut x: i32 = 5;
pub fn parse_let_mut_binding(input: &str) -> Result<(String, String, String), String> {
    let input = input.trim();
    
    if !input.starts_with("let mut ") {
        return Err("mutable variable definition must start with 'let mut'".to_string());
    }

    let after_let_mut = &input[8..];
    
    // Find the colon
    let colon_pos = after_let_mut.find(':')
        .ok_or("missing ':' in let mut binding")?;
    
    let name = after_let_mut[..colon_pos].trim().to_string();
    
    if name.is_empty() {
        return Err("variable name cannot be empty".to_string());
    }

    // Rest is same as let binding
    let after_colon = &after_let_mut[colon_pos + 1..];
    let eq_pos = after_colon.find('=')
        .ok_or("missing '=' in let mut binding")?;
    
    let var_type = after_colon[..eq_pos].trim().to_string();
    
    let after_eq = &after_colon[eq_pos + 1..].trim();
    let value = if after_eq.ends_with(';') {
        &after_eq[..after_eq.len() - 1]
    } else {
        after_eq
    }.trim().to_string();

    Ok((name, var_type, value))
}

/// Parse a simple expression
/// For now, just validate it's not empty
pub fn parse_expression(input: &str) -> Result<String, String> {
    let input = input.trim();
    
    if input.is_empty() {
        return Err("expression cannot be empty".to_string());
    }

    // Basic validation
    if input.contains(";;") {
        return Err("double semicolon in expression".to_string());
    }

    Ok(input.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let input = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let result = parse_function_def(input).unwrap();
        assert_eq!(result.name, "add");
        assert_eq!(result.return_type, "i32");
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_parse_no_param_function() {
        let input = "fn get_five() -> i32 { 5 }";
        let result = parse_function_def(input).unwrap();
        assert_eq!(result.name, "get_five");
        assert_eq!(result.params.len(), 0);
    }

    #[test]
    fn test_parse_let_binding() {
        let input = "let x: i32 = 5;";
        let (name, ty, value) = parse_let_binding(input).unwrap();
        assert_eq!(name, "x");
        assert_eq!(ty, "i32");
        assert_eq!(value, "5");
    }

    #[test]
    fn test_parse_let_string() {
        let input = "let msg: String = \"hello\";";
        let (name, ty, value) = parse_let_binding(input).unwrap();
        assert_eq!(name, "msg");
        assert_eq!(ty, "String");
        assert_eq!(value, "\"hello\"");
    }

    #[test]
    fn test_parse_let_mut_binding() {
        let input = "let mut x: i32 = 5;";
        let (name, ty, value) = parse_let_mut_binding(input).unwrap();
        assert_eq!(name, "x");
        assert_eq!(ty, "i32");
        assert_eq!(value, "5");
    }

    #[test]
    fn test_parse_expression() {
        let input = "x + y";
        let result = parse_expression(input).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_function_error_no_fn() {
        let result = parse_function_def("let x = 5;");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_binding_error_no_let() {
        let result = parse_let_binding("fn x() {}");
        assert!(result.is_err());
    }
}
