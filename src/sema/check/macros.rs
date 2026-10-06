//! The built-in macros.
//!
//! Each macro call is rewritten into ordinary AST that uses the standard
//! library, and that AST is then type-checked like hand-written code:
//!
//! ```text
//! println!("{} is {:?}", name, value)
//!
//! match (&name, &value) {
//!     (__arg0, __arg1) => {
//!         let mut __buffer = std::fmt::Formatter::new();
//!         let __fmt = &mut __buffer;
//!         std::fmt::Display::fmt(__arg0, __fmt);
//!         __fmt.write_str(" is ");
//!         std::fmt::Debug::fmt(__arg1, __fmt);
//!         __fmt.write_str("\n");
//!         std::io::_print(__fmt.finish())
//!     }
//! }
//! ```
//!
//! So formatting any type is nothing more than a trait method call. The
//! arguments are bound by a `match`, as `format_args!` does, so that what
//! they borrow from (`text.trim()` of a temporary `String`) lives until the
//! end of the statement.

use super::FnCtxt;
use crate::sema::defs::Def;
use crate::sema::thir::{Expr, Stmt};
use crate::sema::ty::Ty;
use crate::syntax::ast::{self, BinOp, MacroArgs};
use crate::syntax::build::AstBuilder;
use crate::syntax::diagnostic::{bail, Result};
use crate::syntax::span::Span;

impl FnCtxt<'_, '_> {
    pub fn check_macro(&mut self, call: &ast::MacroCall, expected: Option<&Ty>, span: Span) -> Result<Expr> {
        if let ("vec", MacroArgs::List(elements)) = (call.name.name.as_str(), &call.args) {
            return self.check_vec_macro(elements, expected, span);
        }
        let expansion = self.expand_macro(call, span)?;
        self.check_expr(&expansion, expected)
    }

    /// `vec![a, b, c]`: a vector with room for the elements, then one `push`
    /// each. The vector's type is settled from the expected type *before*
    /// the elements are checked, so each one is coerced to the element type
    /// (`vec![Box::new(Circle), Box::new(Square)]` as `Vec<Box<dyn Shape>>`).
    fn check_vec_macro(&mut self, elements: &[ast::Expr], expected: Option<&Ty>, span: Span) -> Result<Expr> {
        let b = AstBuilder::new(span);
        let Some(vec) = self.tcx.lang.vec else { bail!(span, "the standard library's `Vec` is missing") };
        let vec_ty = Ty::Adt(vec, vec![self.infer.fresh_var()]);
        if let Some(expected) = expected {
            self.infer.try_unify(&vec_ty, expected);
        }

        self.in_scope(|fcx| {
            let capacity = b.int(elements.len() as u128, "usize");
            let allocate = b.call_path(&["std", "vec", "Vec", "with_capacity"], vec![capacity]);
            let allocate = fcx.check_expr_coerce(&allocate, &vec_ty)?;
            let vector = fcx.declare_local("__vec", vec_ty.clone(), true);
            let mut stmts = vec![fcx.let_local(vector, allocate)];
            for element in elements {
                let push = b.method(b.var("__vec"), "push", vec![element.clone()]);
                stmts.push(Stmt::Expr(fcx.check_expr(&push, None)?));
            }
            let result = fcx.local_expr(vector, span);
            Ok(fcx.block_expr(stmts, Some(result), vec_ty.clone(), span))
        })
    }

    fn expand_macro(&mut self, call: &ast::MacroCall, span: Span) -> Result<ast::Expr> {
        let b = AstBuilder::new(span);
        let name = call.name.name.as_str();

        let args = match &call.args {
            MacroArgs::List(args) => args.as_slice(),
            MacroArgs::Repeat(element, count) if name == "vec" => {
                return Ok(b.call_path(&["std", "vec", "from_elem"], vec![(**element).clone(), (**count).clone()]));
            }
            MacroArgs::Matches(value, pattern, guard) if name == "matches" => {
                let arm = |pat, guard, value| ast::Arm { pat, guard, body: b.bool(value) };
                let otherwise = ast::Pattern { kind: ast::PatternKind::Wild, span };
                let arms = vec![
                    arm((**pattern).clone(), guard.as_deref().cloned(), true),
                    arm(otherwise, None, false),
                ];
                return Ok(ast::Expr { kind: ast::ExprKind::Match { scrutinee: value.clone(), arms }, span });
            }
            _ => bail!(call.name.span, "invalid arguments for `{name}!`"),
        };

        Ok(match name {
            "format" => {
                let (arguments, stmts, _) = self.format_statements(&b, args, span)?;
                let stmts = [new_formatter(&b), stmts].concat();
                b.bind(arguments, b.block(stmts, Some(b.method(b.var("__fmt"), "finish", vec![]))))
            }
            "format_args" => b.call_path(&["std", "fmt", "Arguments", "new"], vec![format_call(&b, args.to_vec(), span)]),
            "print" | "println" | "eprint" | "eprintln" => {
                let newline = name.ends_with("ln");
                let sink = if name.starts_with('e') { "_eprint" } else { "_print" };
                if args.is_empty() {
                    let text = if newline { "\n" } else { "" };
                    return Ok(b.call_path(&["std", "io", &format!("{sink}_str")], vec![b.str(text)]));
                }
                let (arguments, stmts, pieces) = self.format_statements(&b, args, span)?;
                // Nothing to format: print the text directly, with no buffer.
                if let [Piece::Text(text)] = pieces.as_slice() {
                    let text = if newline { format!("{text}\n") } else { text.clone() };
                    return Ok(b.call_path(&["std", "io", &format!("{sink}_str")], vec![b.str(&text)]));
                }
                let mut stmts = [new_formatter(&b), stmts].concat();
                if newline {
                    stmts.push(b.stmt(b.method(b.var("__fmt"), "write_str", vec![b.str("\n")])));
                }
                let text = b.method(b.var("__fmt"), "finish", vec![]);
                b.bind(arguments, b.block(stmts, Some(b.call_path(&["std", "io", sink], vec![text]))))
            }
            "write" | "writeln" => {
                let Some((destination, format_args)) = args.split_first() else {
                    bail!(span, "`{name}!` needs a destination and a format string");
                };
                // Anything but a `Formatter` gets the text through its
                // `write_fmt`: a `String` by `fmt::Write`, a file by `io::Write`.
                if !self.is_formatter(destination)? {
                    let mut format_args = format_args.to_vec();
                    if name == "writeln" {
                        match format_args.first_mut() {
                            Some(ast::Expr { kind: ast::ExprKind::Str(template), .. }) => template.push('\n'),
                            Some(_) => bail!(span, "the format string must be a string literal"),
                            None => format_args.push(b.str("\n")),
                        }
                    }
                    let text = format_call(&b, format_args, span);
                    let arguments = b.call_path(&["std", "fmt", "Arguments", "new"], vec![text]);
                    return Ok(b.method(destination.clone(), "write_fmt", vec![arguments]));
                }
                let (arguments, mut stmts, _) = self.format_statements(&b, format_args, span)?;
                // The destination is itself a `&mut Formatter`, whose own
                // placeholder options do not apply to the ones written here.
                stmts.insert(0, b.let_("__fmt", false, b.addr_of_mut(b.deref(destination.clone()))));
                stmts.insert(1, b.let_("__saved", false, b.method(b.var("__fmt"), "take_spec", vec![])));
                if name == "writeln" {
                    stmts.push(b.stmt(b.method(b.var("__fmt"), "write_str", vec![b.str("\n")])));
                }
                stmts.push(b.stmt(b.method(b.var("__fmt"), "restore_spec", vec![b.var("__saved")])));
                b.bind(arguments, b.block(stmts, Some(b.call_path(&["std", "result", "Result", "Ok"], vec![b.unit()]))))
            }
            "panic" => self.panic_call(&b, args, "explicit panic", span)?,
            "todo" => self.panic_call(&b, args, "not yet implemented", span)?,
            "unimplemented" => self.panic_call(&b, args, "not implemented", span)?,
            "unreachable" => self.panic_call(&b, args, "internal error: entered unreachable code", span)?,
            "assert" | "debug_assert" => {
                let Some((condition, message)) = args.split_first() else {
                    bail!(span, "`{name}!` needs a condition");
                };
                let default = format!("assertion failed: {}", self.tcx.sources.snippet(condition.span));
                let failure = self.panic_call(&b, message, &default, span)?;
                b.if_(b.not(condition.clone()), vec![b.stmt(failure)])
            }
            "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => {
                let [left, right, ..] = args else { bail!(span, "`{name}!` needs two values to compare") };
                let (op, symbol) = if name.ends_with("eq") { (BinOp::Eq, "==") } else { (BinOp::Ne, "!=") };
                let holds = b.binary(op, b.deref(b.var("__left")), b.deref(b.var("__right")));
                let debug = |value: &str| {
                    ast::Expr {
                        kind: ast::ExprKind::Macro(ast::MacroCall {
                            name: b.ident("format"),
                            args: MacroArgs::List(vec![b.str("{:?}"), b.var(value)]),
                        }),
                        span,
                    }
                };
                let report = b.call_path(
                    &["std", "rt", "assert_failed"],
                    vec![b.str(symbol), debug("__left"), debug("__right"), b.str(&self.location(span))],
                );
                b.bind(
                    vec![("__left".into(), b.addr_of(left.clone())), ("__right".into(), b.addr_of(right.clone()))],
                    b.block(vec![b.stmt(b.if_(b.not(holds), vec![b.stmt(report)]))], None),
                )
            }
            "dbg" => {
                let [value] = args else { bail!(span, "`dbg!` takes exactly one value") };
                let label = format!("[{}] {} = ", self.location(span), self.tcx.sources.snippet(value.span));
                let line = ast::Expr {
                    kind: ast::ExprKind::Macro(ast::MacroCall {
                        name: b.ident("eprintln"),
                        args: MacroArgs::List(vec![b.str("{}{:?}"), b.str(&label), b.addr_of(b.var("__value"))]),
                    }),
                    span,
                };
                b.block(vec![b.let_("__value", false, value.clone()), b.stmt(line)], Some(b.var("__value")))
            }
            "stringify" => {
                let text: Vec<&str> = args.iter().map(|arg| self.tcx.sources.snippet(arg.span)).collect();
                b.str(&text.join(", "))
            }
            "concat" => {
                let mut text = String::new();
                for arg in args {
                    match &arg.kind {
                        ast::ExprKind::Str(piece) => text.push_str(piece),
                        ast::ExprKind::Char(piece) => text.push(*piece),
                        ast::ExprKind::Bool(_)
                        | ast::ExprKind::Int(..)
                        | ast::ExprKind::Float(..)
                        | ast::ExprKind::Unary(ast::UnOp::Neg, _) => text.push_str(self.tcx.sources.snippet(arg.span)),
                        _ => bail!(arg.span, "`concat!` takes only literals"),
                    }
                }
                b.str(&text)
            }
            "file" => b.str(&self.tcx.sources.locate(span).path.display().to_string()),
            "line" => b.int(self.tcx.sources.locate(span).line as u128, "u32"),
            "column" => b.int(self.tcx.sources.locate(span).column as u128, "u32"),
            "module_path" => b.str(&self.tcx.module_path(self.module)),
            _ => bail!(call.name.span, "cannot find macro `{name}!`"),
        })
    }

    /// Is `destination` a `Formatter` (or a reference to one)? Found by
    /// checking it on trial and taking back what that inferred.
    fn is_formatter(&mut self, destination: &ast::Expr) -> Result<bool> {
        let Some(Def::Adt(formatter)) = self.tcx.defs.std_item(&["fmt", "Formatter"]) else {
            return Ok(false);
        };
        let snapshot = self.infer.snapshot();
        let checked = self.check_expr(destination, None);
        let ty = checked.map(|expr| self.resolve(&expr.ty));
        self.infer.rollback_to(snapshot);
        let mut ty = ty?;
        while let Ty::Ref(inner, _) = ty {
            ty = *inner;
        }
        Ok(matches!(ty, Ty::Adt(adt, _) if adt == formatter))
    }

    /// `file:line:column` of a span, for panic messages.
    fn location(&self, span: Span) -> String {
        let at = self.tcx.sources.locate(span);
        format!("{}:{}:{}", at.path.display(), at.line, at.column)
    }

    /// A call that reports a panic and never returns. `args` is an optional
    /// format string with arguments; without it, `default` is the message.
    fn panic_call(&mut self, b: &AstBuilder, args: &[ast::Expr], default: &str, span: Span) -> Result<ast::Expr> {
        let location = b.str(&self.location(span));
        if args.is_empty() {
            return Ok(b.call_path(&["std", "rt", "panic_str"], vec![b.str(default), location]));
        }
        // A message with nothing to format is the payload itself, a `&'static str`.
        if let [message @ ast::Expr { kind: ast::ExprKind::Str(text), .. }] = args {
            if !text.contains(['{', '}']) {
                return Ok(b.call_path(&["std", "rt", "panic_str"], vec![message.clone(), location]));
            }
        }
        let message = ast::Expr {
            kind: ast::ExprKind::Macro(ast::MacroCall { name: b.ident("format"), args: MacroArgs::List(args.to_vec()) }),
            span,
        };
        Ok(b.call_path(&["std", "rt", "panic_fmt"], vec![message, location]))
    }

    /// The statements that write a format string and its arguments through
    /// `__fmt`, a `&mut Formatter` the caller declares. They refer to the
    /// arguments as `__arg0`, `__arg1`, ..., which the caller binds to the
    /// borrows returned first, with [`AstBuilder::bind`].
    #[allow(clippy::type_complexity)]
    fn format_statements(
        &mut self,
        b: &AstBuilder,
        args: &[ast::Expr],
        span: Span,
    ) -> Result<(Vec<(String, ast::Expr)>, Vec<ast::Stmt>, Vec<Piece>)> {
        let Some((template, values)) = args.split_first() else {
            bail!(span, "expected a format string");
        };
        let ast::ExprKind::Str(template_text) = &template.kind else {
            bail!(template.span, "the format string must be a string literal");
        };
        let pieces = parse_format_string(template_text, template.span)?;

        // Evaluate every argument exactly once, in order, before formatting.
        let mut arguments = Vec::new();
        let mut stmts = Vec::new();
        let mut named = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let value = match &value.kind {
                ast::ExprKind::Assign(target, value) => match &target.kind {
                    ast::ExprKind::Path(path) if path.as_ident().is_some() => {
                        named.push((path.as_ident().unwrap().name.clone(), index));
                        value
                    }
                    _ => value,
                },
                _ => value,
            };
            arguments.push((format!("__arg{index}"), b.addr_of(value.clone())));
        }

        // An argument used as a width or precision is a `usize`. Saying so
        // before anything is formatted lets that decide the type of a number
        // literal that is also printed (`{:>1$}` with `let n = 4;`).
        let mut next = 0;
        for piece in &pieces {
            let Piece::Argument(argument, spec) = piece else { continue };
            for count in [&spec.precision, &spec.width] {
                let index = match count {
                    Some(Count::Argument(Argument::Next)) => {
                        next += 1;
                        Some(next - 1)
                    }
                    Some(Count::Argument(Argument::Position(index))) => Some(*index),
                    Some(Count::Argument(Argument::Named(name))) => named.iter().find(|(n, _)| n == name).map(|(_, index)| *index),
                    _ => None,
                };
                if let Some(index) = index.filter(|&index| index < values.len()) {
                    let count = b.call_path(&["std", "fmt", "count_argument"], vec![b.deref(b.var(&format!("__arg{index}")))]);
                    stmts.push(b.stmt(count));
                }
            }
            if matches!(argument, Argument::Next) {
                next += 1;
            }
        }

        // Which argument a placeholder (or a width or precision taken from
        // an argument) refers to, marking it used.
        let mut next_positional = 0;
        let mut used = vec![false; values.len()];
        let mut resolve = |argument: &Argument| -> Result<ast::Expr> {
            let index = match argument {
                Argument::Next => {
                    next_positional += 1;
                    Some(next_positional - 1)
                }
                Argument::Position(index) => Some(*index),
                Argument::Named(name) => named.iter().find(|(n, _)| n == name).map(|(_, index)| *index),
            };
            match (index, argument) {
                (Some(index), _) if index < values.len() => {
                    used[index] = true;
                    Ok(b.var(&format!("__arg{index}")))
                }
                // `{name}` with no matching argument captures a variable in scope.
                (None, Argument::Named(name)) => Ok(b.addr_of(b.var(name))),
                _ => bail!(template.span, "the format string refers to an argument that was not supplied"),
            }
        };

        for piece in &pieces {
            let (argument, spec) = match piece {
                Piece::Text(text) => {
                    stmts.push(b.stmt(b.method(b.var("__fmt"), "write_str", vec![b.str(text)])));
                    continue;
                }
                Piece::Argument(argument, spec) => (argument, spec),
            };
            // `{:.*}` takes its precision from the argument before the value.
            let mut count = |count: &Option<Count>| -> Result<ast::Expr> {
                Ok(match count {
                    None => b.int(0, "usize"),
                    Some(Count::Literal(value)) => b.int(*value as u128, "usize"),
                    Some(Count::Argument(argument)) => b.deref(resolve(argument)?),
                })
            };
            let precision = count(&spec.precision)?;
            let width = count(&spec.width)?;
            let value = resolve(argument)?;

            if !spec.is_default() {
                let settings = vec![
                    b.char(spec.fill),
                    b.int(spec.align as u128, "u8"),
                    b.int(spec.flags() as u128, "u32"),
                    width,
                    precision,
                ];
                stmts.push(b.stmt(b.method(b.var("__fmt"), "set_spec", settings)));
            }
            let format = b.call_path(&["std", "fmt", spec.trait_name, "fmt"], vec![value, b.var("__fmt")]);
            stmts.push(b.stmt(format));
            if !spec.is_default() {
                stmts.push(b.stmt(b.method(b.var("__fmt"), "clear_spec", vec![])));
            }
        }
        if let Some(unused) = used.iter().position(|used| !used) {
            bail!(values[unused].span, "this argument is never used by the format string");
        }
        Ok((arguments, stmts, pieces))
    }
}

/// `format!(args...)` as an expression.
fn format_call(b: &AstBuilder, args: Vec<ast::Expr>, span: Span) -> ast::Expr {
    ast::Expr { kind: ast::ExprKind::Macro(ast::MacroCall { name: b.ident("format"), args: MacroArgs::List(args) }), span }
}

/// `let mut __buffer = Formatter::new(); let __fmt = &mut __buffer;`
fn new_formatter(b: &AstBuilder) -> Vec<ast::Stmt> {
    vec![
        b.let_("__buffer", true, b.call_path(&["std", "fmt", "Formatter", "new"], vec![])),
        b.let_("__fmt", false, b.addr_of_mut(b.var("__buffer"))),
    ]
}

// ---------------------------------------------------------------------------
// Format strings
// ---------------------------------------------------------------------------

enum Piece {
    Text(String),
    Argument(Argument, FormatSpec),
}

enum Argument {
    /// `{}` — the next positional argument.
    Next,
    /// `{0}`
    Position(usize),
    /// `{name}`
    Named(String),
}

/// A width or precision: written out, or taken from an argument
/// (`{:>1$}`, `{:>width$}`, `{:.*}`).
enum Count {
    Literal(usize),
    Argument(Argument),
}

/// What comes after the `:` in `{:>8.2}`.
struct FormatSpec {
    /// Which formatting trait renders the value.
    trait_name: &'static str,
    fill: char,
    /// 0 = unspecified, 1 = left, 2 = centre, 3 = right.
    align: u8,
    sign_plus: bool,
    alternate: bool,
    zero_pad: bool,
    width: Option<Count>,
    precision: Option<Count>,
}

impl FormatSpec {
    fn is_default(&self) -> bool {
        self.flags() == 0 && self.align == 0 && self.fill == ' '
    }

    /// The option bits handed to `Formatter::set_spec`.
    fn flags(&self) -> u32 {
        (self.sign_plus as u32)
            | (self.alternate as u32) << 1
            | (self.zero_pad as u32) << 2
            | (self.width.is_some() as u32) << 3
            | (self.precision.is_some() as u32) << 4
    }
}

fn parse_format_string(template: &str, span: Span) -> Result<Vec<Piece>> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                text.push('}');
            }
            '{' => {
                let mut inside = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(c) => inside.push(c),
                        None => bail!(span, "unterminated `{{` in format string"),
                    }
                }
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(parse_placeholder(&inside, span)?);
            }
            '}' => bail!(span, "unmatched `}}` in format string (write `}}}}` for a literal brace)"),
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    Ok(pieces)
}

/// The inside of one `{...}`: `argument:spec`, both optional.
fn parse_placeholder(inside: &str, span: Span) -> Result<Piece> {
    let (argument, spec) = inside.split_once(':').unwrap_or((inside, ""));
    let argument = match argument.trim() {
        "" => Argument::Next,
        name => match name.parse() {
            Ok(index) => Argument::Position(index),
            Err(_) => Argument::Named(name.to_string()),
        },
    };

    let mut spec_chars: Vec<char> = spec.chars().collect();
    let mut parsed = FormatSpec {
        trait_name: "Display",
        fill: ' ',
        align: 0,
        sign_plus: false,
        alternate: false,
        zero_pad: false,
        width: None,
        precision: None,
    };

    // The trait is named last: `?`, `x`, `X`, `b`, `o`, `e`.
    parsed.trait_name = match spec_chars.last() {
        Some('?') => "Debug",
        Some('x') => "LowerHex",
        Some('X') => "UpperHex",
        Some('b') => "Binary",
        Some('o') => "Octal",
        Some('e') => "LowerExp",
        Some('E') => "UpperExp",
        _ => "Display",
    };
    if parsed.trait_name != "Display" {
        spec_chars.pop();
    }

    let align_of = |c: char| match c {
        '<' => Some(1),
        '^' => Some(2),
        '>' => Some(3),
        _ => None,
    };
    let mut rest = spec_chars.as_slice();
    // `[[fill]align]`
    if let [fill, align, tail @ ..] = rest {
        if let Some(align) = align_of(*align) {
            (parsed.fill, parsed.align) = (*fill, align);
            rest = tail;
        }
    }
    if parsed.align == 0 {
        if let [align, tail @ ..] = rest {
            if let Some(align) = align_of(*align) {
                parsed.align = align;
                rest = tail;
            }
        }
    }
    if let ['+', tail @ ..] = rest {
        parsed.sign_plus = true;
        rest = tail;
    }
    if let ['#', tail @ ..] = rest {
        parsed.alternate = true;
        rest = tail;
    }
    if let ['0', tail @ ..] = rest {
        parsed.zero_pad = true;
        rest = tail;
    }

    let remainder: String = rest.iter().collect();
    let (width, precision) = match remainder.split_once('.') {
        Some((width, precision)) => (width, Some(precision)),
        None => (remainder.as_str(), None),
    };
    let count = |text: &str| -> Result<Option<Count>> {
        if text.is_empty() {
            return Ok(None);
        }
        if let Ok(value) = text.parse() {
            return Ok(Some(Count::Literal(value)));
        }
        match text.strip_suffix('$') {
            Some(name) if !name.is_empty() => Ok(Some(Count::Argument(match name.parse() {
                Ok(index) => Argument::Position(index),
                Err(_) => Argument::Named(name.to_string()),
            }))),
            _ => bail!(span, "unsupported format specification `{{:{spec}}}`"),
        }
    };
    parsed.width = count(width)?;
    parsed.precision = match precision {
        Some("*") => Some(Count::Argument(Argument::Next)),
        Some(precision) => count(precision)?,
        None => None,
    };
    Ok(Piece::Argument(argument, parsed))
}
