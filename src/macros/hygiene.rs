use std::collections::HashMap;
use crate::lexer::token::Token;
use super::TokenTree;

#[derive(Debug, Clone)]
pub struct HygieneContext {
    next_gensym: usize,
    renaming_map: HashMap<String, String>,
    scopes: Vec<HashMap<String, String>>,
}

impl HygieneContext {
    pub fn new() -> Self {
        HygieneContext {
            next_gensym: 0,
            renaming_map: HashMap::new(),
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn gensym(&mut self, base: &str) -> String {
        let gensym = format!("{}__gensym_{}", base, self.next_gensym);
        self.next_gensym += 1;
        gensym
    }

    pub fn rename_ident(&mut self, original: &str) -> String {
        let renamed = self.gensym(original);
        self.renaming_map.insert(original.to_string(), renamed.clone());
        
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(original.to_string(), renamed.clone());
        }
        
        renamed
    }

    pub fn get_renamed(&self, original: &str) -> Option<String> {
        if let Some(scope) = self.scopes.last() {
            scope.get(original).cloned()
        } else {
            None
        }
    }

    pub fn apply_hygiene(&mut self, tokens: &[TokenTree]) -> Vec<TokenTree> {
        tokens.iter()
            .map(|tree| self.hygiene_token_tree(tree))
            .collect()
    }

    fn hygiene_token_tree(&mut self, tree: &TokenTree) -> TokenTree {
        match tree {
            TokenTree::Token(token) => {
                TokenTree::Token(self.hygiene_token(token))
            }
            TokenTree::Group { delimiter, stream } => {
                self.push_scope();
                let new_stream = self.apply_hygiene(stream);
                self.pop_scope();
                TokenTree::Group {
                    delimiter: *delimiter,
                    stream: new_stream,
                }
            }
        }
    }

    fn hygiene_token(&mut self, token: &Token) -> Token {
        match token {
            Token::Identifier(name) => {
                if let Some(renamed) = self.get_renamed(name) {
                    Token::Identifier(renamed)
                } else if self.is_user_defined_ident(name) {
                    let renamed = self.rename_ident(name);
                    Token::Identifier(renamed)
                } else {
                    token.clone()
                }
            }
            _ => token.clone(),
        }
    }

    fn is_user_defined_ident(&self, ident: &str) -> bool {
        !matches!(
            ident,
            "i32" | "i64" | "i16" | "i8" | "u32" | "u64" | "u16" | "u8"
                | "f32" | "f64" | "bool" | "char" | "str" | "usize" | "isize"
                | "Self" | "self" | "super" | "crate" | "true" | "false"
        )
    }
}

pub fn apply_macro_hygiene(
    expansion: &[TokenTree],
    _macro_name: &str,
) -> Vec<TokenTree> {
    let mut context = HygieneContext::new();
    
    for tree in expansion {
        match tree {
            TokenTree::Token(Token::Identifier(name)) 
                if !is_builtin_or_imported(name) => {
                context.rename_ident(name);
            }
            TokenTree::Group { stream, .. } => {
                for inner_tree in stream {
                    if let TokenTree::Token(Token::Identifier(name)) = inner_tree {
                        if !is_builtin_or_imported(name) {
                            context.rename_ident(name);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    context.apply_hygiene(expansion)
}

fn is_builtin_or_imported(name: &str) -> bool {
    matches!(
        name,
        "i32" | "i64" | "i16" | "i8" | "u32" | "u64" | "u16" | "u8"
            | "f32" | "f64" | "bool" | "char" | "str" | "usize" | "isize"
            | "String" | "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc"
            | "RefCell" | "Cell" | "Mutex" | "RwLock" | "HashMap" | "HashSet"
            | "BTreeMap" | "BTreeSet" | "LinkedList" | "VecDeque" | "BinaryHeap"
            | "println" | "print" | "eprintln" | "panic" | "assert" | "assert_eq"
            | "assert_ne" | "debug_assert" | "debug_assert_eq" | "debug_assert_ne"
            | "format" | "vec" | "Some" | "None" | "Ok" | "Err"
    )
}
