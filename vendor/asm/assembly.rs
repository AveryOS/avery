#![crate_type = "dylib"]
#![crate_name = "assembly"]
#![feature(plugin_registrar, rustc_private, str_char)]
#![allow()]

extern crate rustc_plugin;
extern crate syntax;

use std::collections::HashMap;
use syntax::ptr::P;
use syntax::ast;
use syntax::ast::{Expr, AsmDialect};
use syntax::codemap;
use syntax::codemap::Pos;
use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use rustc_plugin::registry::Registry;
use self::asm::expand_asm;
use syntax::parse::parser::{LhsExpr, Parser};
use syntax::parse::token;
use syntax::parse::token::{keywords, intern_and_get_ident};
use syntax::parse::common::SeqSep;
use syntax::print::pprust::token_to_string;
use syntax::util::parser::AssocOp;
use syntax::parse::PResult;

macro_rules! panictry {
    ($e:expr) => ({
        use std::result::Result::{Ok, Err};
        use syntax::errors::FatalError;
        match $e {
            Ok(e) => e,
            Err(mut e) => {
                e.emit();
                panic!(FatalError);
            }
        }
    })
}

mod asm;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("asm", expand);
}

enum BindingKind {
    Bare,
    Input(P<Expr>),
    Output(P<Expr>),
    InputThenOutput(P<Expr>, P<Expr>),
    InputAndOutput(P<Expr>)
}

enum BindingIdx {
    Input(usize),
    Output(usize)
}

struct Constraint {
    name: String,
    indirect: bool,
    early_clobber: bool
}

struct Binding {
    constraint: Constraint,
    kind: Option<BindingKind>,
    t_idx: BindingIdx,
    idx: usize
}

struct Data {
    dialect: AsmDialect,
    alignstack: bool,
    volatile: bool,
    clobbers: Vec<String>,
    idents: HashMap<String, usize>,
    bindings: Vec<Binding>
}

fn format_c(c: &Constraint, input: bool) -> token::InternedString {
    let mut base = if c.name.len() > 1 {
        format!("{{{}}}", c.name)
    } else {
        c.name.clone()
    };

    if c.indirect {
        base = "*".to_string() + &base;
    }

    let result = if input {
        base
    } else if c.early_clobber {
        "=&".to_string() + &base
    } else {
        "=".to_string() + &base
    };

    intern_and_get_ident(&result)
}

fn add_binding(data: &mut Data, name: Option<String>, c: Constraint, kind: BindingKind) -> usize {
    let idx = data.bindings.len();
    data.bindings.push(Binding {
        constraint: c,
        kind: Some(kind),
        t_idx: BindingIdx::Input(0),
        idx: 0
    });
    name.map(|name| {
        data.idents.insert(name, idx);
    });
    idx
}

fn get_ident<'a>(p: &mut Parser<'a>) -> PResult<'a, String> {
    match p.token {
        token::Ident(id, _) => {
            p.bump();
            Ok(id.name.as_str().to_string())
        }
        _ => {
            p.unexpected()
        }
    }
}

fn parse_c<'a>(p: &mut Parser<'a>) -> PResult<'a, Constraint> {
    try!(p.expect(&token::BinOp(token::Percent)));
    let early_clobber = p.eat(&token::BinOp(token::And));
    let indirect = p.eat(&token::BinOp(token::Star));
    Ok(Constraint { name: try!(get_ident(p)), indirect: indirect, early_clobber: early_clobber })
}

fn parse_let<'a>(p: &mut Parser<'a>) -> PResult<'a, String> {
    try!(p.expect_keyword(keywords::Let));
    let n = try!(get_ident(p));
    try!(p.expect(&token::Colon));
    Ok(n)
}

fn parse_c_arrow<'a>(p: &mut Parser<'a>, data: &mut Data) -> PResult<'a, usize> {
    let c = try!(parse_c(p));

    let rw = if p.eat(&token::Le) {
        try!(p.expect(&token::Gt));
        true
    } else {
        try!(p.expect(&token::FatArrow));
        false
    };

    let e = try!(p.parse_expr());

    let kind = if rw {
        BindingKind::InputAndOutput(e)
    } else {
        BindingKind::Output(e)
    };

    Ok(add_binding(data, None, c, kind))
}

fn parse_operand<'a>(p: &mut Parser<'a>, data: &mut Data) -> PResult<'a, usize> {
    match p.token {
        token::BinOp(token::Percent) => {
            parse_c_arrow(p, data)
        }
        _ => {
            let exp = try!(p.parse_assoc_expr_with(AssocOp::BitOr.precedence(), LhsExpr::NotYetParsed));

            let rw = if p.eat(&token::Le) {
                try!(p.expect(&token::Gt));
                true
            } else {
                try!(p.expect(&token::FatArrow));
                false
            };

            let name = match p.token {
                token::Ident(_, _) => Some(try!(parse_let(p))),
                _ => None
            };

            let c = try!(parse_c(p));

            let kind = if rw {
                BindingKind::InputAndOutput(exp)
            } else if p.eat(&token::FatArrow) {
                BindingKind::InputThenOutput(exp, try!(p.parse_expr()))
            }  else {
                BindingKind::Input(exp)
            };

            Ok(add_binding(data, name, c, kind))
        }
    }
}

fn parse_binding<'a>(p: &mut Parser<'a>, data: &mut Data) -> PResult<'a, usize> {
    if p.token.is_keyword(keywords::Let) {
        let name = Some(try!(parse_let(p)));
        let c = try!(parse_c(p));

        let kind = if p.eat(&token::FatArrow) {
            BindingKind::Output(try!(p.parse_expr()))
        } else {
            BindingKind::Bare
        };

        Ok(add_binding(data, name, c, kind))
    } else {
        parse_operand(p, data)
    }
}

fn parse_opt<'a>(cx: &mut ExtCtxt, p: &mut Parser<'a>, data: &mut Data) -> PResult<'a, ()> {
    if p.token.is_keyword(keywords::Use) {
        p.bump();
        data.clobbers.push(format!("~{{{}}}", &try!(get_ident(p))));
    } else if p.token.is_keyword(keywords::Mod) {
        p.bump();
        match &try!(get_ident(p))[..] {
            "attsyntax" => {
                data.dialect = AsmDialect::Att;
            }
            "alignstack" => {
                data.alignstack = true;
            }
            "pure" => {
                data.volatile = false;
            }
            _ => cx.span_err(p.last_span, "unknown option")
        };
    } else {
        try!(parse_binding(p, data));
    }

    Ok(())
}

enum Output {
    Str(String),
    Binding(usize)
}

fn search<F: Fn(usize) -> bool>(fb: &codemap::FileMapAndBytePos, test: F, offset: isize) -> Option<u8> {
    let mut p = fb.pos.to_usize();

    loop {
        if test(p) {
            return None;
        }

        p = match offset {
            -1 => p - 1,
            1 => p + 1,
            _ => panic!()
        };

        let c = fb.fm.src.as_ref().unwrap()[..].as_bytes()[p];

        if (c & 0xC0) != 0x80 {
            return Some(c)
        }
    }
}

fn is_whitespace(c: Option<u8>) -> bool {
    match c {
        Some(c) => {
            match c as char {
                '\t' | ' ' | '\n' | '\r' => true,
                _ => false
            }
        }
        None => true
    }
}

fn is_whitespace_left(cx: &ExtCtxt, sp: codemap::Span) -> bool {
    let cm = cx.parse_sess.codemap();
    let fb = cm.lookup_byte_offset(sp.lo);
    is_whitespace(search(&fb, |p| { p == 0 }, -1))
}

fn is_whitespace_right(cx: &ExtCtxt, sp: codemap::Span) -> bool {
    let cm = &cx.parse_sess.codemap();
    let mut fb = cm.lookup_byte_offset(sp.hi);
    fb.pos = codemap::Pos::from_usize(fb.pos.to_usize() - 1);
    is_whitespace(search(&fb, |p| { p >= fb.fm.src.as_ref().unwrap().len() - 1 }, 1))
}

fn whitespace_wrap<'cx, F: FnOnce(&mut Vec<Output>)>(cx: &'cx mut ExtCtxt, out: &mut Vec<Output>, sp: codemap::Span, act: F) {
    if is_whitespace_left(cx, sp) {
        out.push(Output::Str(" ".to_string()));
    }
    act(out);
    if is_whitespace_right(cx, sp) {
        out.push(Output::Str(" ".to_string()));
    }
}

fn expand<'cx>(cx: &'cx mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree]) -> Box<MacResult + 'cx> {
    // Fall back to the old syntax if we start with a string
    if tts.len() > 0 {
        match tts[0] {
            ast::TokenTree::Token(_, token::Literal(token::Lit::Str_(_), _)) | ast::TokenTree::Token(_, token::Literal(token::Lit::StrRaw(_, _), _)) => {
                return expand_asm(cx, sp, tts);
            }
            _ => ()
        }
    }

    let mut p = cx.new_parser_from_tts(tts);

    let mut data = Data { dialect: AsmDialect::Intel, volatile: true, alignstack: false, clobbers: vec!(), idents: HashMap::new(), bindings: vec!() };

    match p.token {
        token::OpenDelim(token::Bracket) => {
            p.parse_unspanned_seq(&token::OpenDelim(token::Bracket),
                                  &token::CloseDelim(token::Bracket),
                                  SeqSep::trailing_allowed(token::Comma),
                                  |p| {
                                       parse_opt(cx, p, &mut data)
                                  }).unwrap();
        }
        _ => ()
    }

    let mut out = vec!(Output::Str("\t".to_string()));

    loop {
        /*println!("Token-left {}", is_whitespace_left(cx, p.span));
        println!("Token-right {}", is_whitespace_right(cx, p.span));
        println!("Token  {}\nspan |{}|",p.token, cx.parse_sess.span_diagnostic.cm.span_to_snippet(p.span).unwrap());*/

        match p.token {
            token::OpenDelim(token::Brace) => {
                    if is_whitespace_left(cx, p.span) {
                        out.push(Output::Str(" ".to_string()));
                    }

                    p.bump();
                    out.push(Output::Binding(parse_binding(&mut p, &mut data).unwrap()));

                    if is_whitespace_right(cx, p.span) {
                        out.push(Output::Str(" ".to_string()));
                    }
                    panictry!(p.expect(&token::CloseDelim(token::Brace)));
            }
            token::Ident(_, _) => {
                whitespace_wrap(cx, &mut out, p.span, |out| {
                    if let Some(idx) = data.idents.get(&token_to_string(&p.token)) {
                        out.push(Output::Binding(*idx));
                        p.bump();
                    } else {
                        out.push(Output::Str(token_to_string(&p.token)));
                        p.bump();
                    }
                });
            }
            token::Dollar => {
                whitespace_wrap(cx, &mut out, p.span, |out| {
                    out.push(Output::Str("$$".to_string()));
                });
                p.bump();
            }
            token::Semi => {
                out.push(Output::Str("\n\t".to_string()));
                p.bump();
            }
            token::Interpolated(_) => {
                panictry!(p.unexpected());
            }
            token::Eof => break,
            _ => {
                whitespace_wrap(cx, &mut out, p.span, |out| {
                    out.push(Output::Str(token_to_string(&p.token)));
                });
                p.bump();
            }
        }
    }

    let mut inputs = vec!();
    let mut outputs = vec!();

    for b in data.bindings.iter_mut() {
        let c_in = format_c(&b.constraint, true);
        let c_out = format_c(&b.constraint, false);

        b.t_idx = match b.kind.take().unwrap() {
            BindingKind::Bare => {
                panic!("Bare unsupported");
                //let c_clobber = intern_and_get_ident(("=&".to_string() + c).as_slice());
                // this needs an expression - outputs.push((c_clobber, , false));
                //BindingIdx::Output(outputs.len())
            }
            BindingKind::Input(e) => {
                inputs.push((c_in, e));
                BindingIdx::Input(inputs.len())
            }
            BindingKind::Output(e) => {
                outputs.push(ast::InlineAsmOutput {
                    constraint: c_out,
                    expr: e,
                    is_rw: false,
                    is_indirect: b.constraint.indirect,
                });
                BindingIdx::Output(outputs.len())
            }
            BindingKind::InputThenOutput(e_in, e_out) => {
                let c_outpos = intern_and_get_ident(&outputs.len().to_string());
                inputs.push((c_outpos, e_in));
                outputs.push(ast::InlineAsmOutput {
                    constraint: c_out,
                    expr: e_out,
                    is_rw: false,
                    is_indirect: b.constraint.indirect,
                });
                BindingIdx::Input(inputs.len());
                BindingIdx::Output(outputs.len())
            }
            BindingKind::InputAndOutput(e) => {
                outputs.push(ast::InlineAsmOutput {
                    constraint: c_out,
                    expr: e,
                    is_rw: true,
                    is_indirect: b.constraint.indirect,
                });
                BindingIdx::Output(outputs.len())
            }
        }
    }

    for b in data.bindings.iter_mut() {
        b.idx = match b.t_idx {
            BindingIdx::Input(i) => outputs.len() + i,
            BindingIdx::Output(i) => i
        } - 1;
    }

    let mut out_str = "".to_string();

    for o in out.into_iter() {
        match o {
            Output::Str(s) => out_str.push_str(&s),
            Output::Binding(idx) => {
                out_str.push_str(" $");
                out_str.push_str(&data.bindings[idx].idx.to_string());
            }
        }
    }

    //println!("Clobbers: {}\nAssembly output: \n{}  \noutputs {}  \ninputs {}", data.clobbers, out_str, outputs, inputs);

    let expn_id = cx.codemap().record_expansion(codemap::ExpnInfo {
        call_site: sp,
        callee: codemap::NameAndSpan {
            format: codemap::MacroBang(token::intern("asm")),
            span: None,
            allow_internal_unstable: false,
        },
    });

    MacEager::expr(P(Expr {
        id: ast::DUMMY_NODE_ID,
        node: ast::ExprKind::InlineAsm(ast::InlineAsm {
            asm: intern_and_get_ident(&out_str),
            asm_str_style: ast::StrStyle::Cooked,
            clobbers: data.clobbers.iter().map(|s| intern_and_get_ident(&s)).collect(),
            inputs: inputs,
            outputs: outputs,
            volatile: data.volatile,
            alignstack: data.alignstack,
            dialect: data.dialect,
            expn_id: expn_id,
        }),
        attrs: None,
        span: sp
    }))
}
