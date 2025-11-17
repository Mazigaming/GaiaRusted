use super::{TokenTree, MacroExpander, MacroRule, MacroPattern, MacroDefinition, MetaVarKind, Delimiter};
use crate::lexer::token::Token;

pub fn register_builtin_macros(expander: &mut MacroExpander) {
    register_println_macro(expander);
    register_print_macro(expander);
    register_vec_macro(expander);
    register_hashmap_macro(expander);
    register_hashset_macro(expander);
    register_format_macro(expander);
    register_assert_macro(expander);
    register_assert_eq_macro(expander);
    register_assert_ne_macro(expander);
    register_panic_macro(expander);
    register_dbg_macro(expander);
    register_eprintln_macro(expander);
}

fn register_println_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_println".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let rule_with_args = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "args".to_string(),
                kind: MetaVarKind::Tt,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_println_args".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("args".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "println".to_string(),
        rules: vec![rule, rule_with_args],
    };

    expander.define(def);
}

fn register_print_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_print".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let rule_with_args = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "args".to_string(),
                kind: MetaVarKind::Tt,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_print_args".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("args".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "print".to_string(),
        rules: vec![rule, rule_with_args],
    };

    expander.define(def);
}

fn register_eprintln_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_eprintln".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let rule_with_args = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "args".to_string(),
                kind: MetaVarKind::Tt,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_eprintln_args".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("args".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "eprintln".to_string(),
        rules: vec![rule, rule_with_args],
    };

    expander.define(def);
}

fn register_vec_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::Group {
                delimiter: Delimiter::Bracket,
                patterns: vec![
                    MacroPattern::MetaVar {
                        name: "item".to_string(),
                        kind: MetaVarKind::Expr,
                    },
                ],
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_vec".to_string())),
            TokenTree::Token(Token::LeftBracket),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("item".to_string())),
            TokenTree::Token(Token::RightBracket),
        ],
    };

    let def = MacroDefinition {
        name: "vec".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_hashmap_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::Group {
                delimiter: Delimiter::Brace,
                patterns: vec![
                    MacroPattern::MetaVar {
                        name: "pairs".to_string(),
                        kind: MetaVarKind::Tt,
                    },
                ],
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_hashmap".to_string())),
            TokenTree::Token(Token::LeftBrace),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("pairs".to_string())),
            TokenTree::Token(Token::RightBrace),
        ],
    };

    let def = MacroDefinition {
        name: "hashmap".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_hashset_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::Group {
                delimiter: Delimiter::Bracket,
                patterns: vec![
                    MacroPattern::MetaVar {
                        name: "items".to_string(),
                        kind: MetaVarKind::Tt,
                    },
                ],
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_hashset".to_string())),
            TokenTree::Token(Token::LeftBracket),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("items".to_string())),
            TokenTree::Token(Token::RightBracket),
        ],
    };

    let def = MacroDefinition {
        name: "hashset".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_format_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_format".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let rule_with_args = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "format_string".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "args".to_string(),
                kind: MetaVarKind::Tt,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_format_args".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("format_string".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("args".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "format".to_string(),
        rules: vec![rule, rule_with_args],
    };

    expander.define(def);
}

fn register_assert_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "condition".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_assert".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("condition".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let rule_with_msg = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "condition".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "message".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_assert_msg".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("condition".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("message".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "assert".to_string(),
        rules: vec![rule, rule_with_msg],
    };

    expander.define(def);
}

fn register_assert_eq_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "left".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "right".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_assert_eq".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("left".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("right".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "assert_eq".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_assert_ne_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "left".to_string(),
                kind: MetaVarKind::Expr,
            },
            MacroPattern::Token(Token::Comma),
            MacroPattern::MetaVar {
                name: "right".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_assert_ne".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("left".to_string())),
            TokenTree::Token(Token::Comma),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("right".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "assert_ne".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_panic_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "message".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_panic".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("message".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "panic".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}

fn register_dbg_macro(expander: &mut MacroExpander) {
    let rule = MacroRule {
        pattern: vec![
            MacroPattern::MetaVar {
                name: "expr".to_string(),
                kind: MetaVarKind::Expr,
            },
        ],
        body: vec![
            TokenTree::Token(Token::Identifier("__builtin_dbg".to_string())),
            TokenTree::Token(Token::LeftParen),
            TokenTree::Token(Token::Dollar),
            TokenTree::Token(Token::Identifier("expr".to_string())),
            TokenTree::Token(Token::RightParen),
        ],
    };

    let def = MacroDefinition {
        name: "dbg".to_string(),
        rules: vec![rule],
    };

    expander.define(def);
}
