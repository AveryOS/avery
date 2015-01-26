#![crate_type = "dylib"]
#![crate_name = "assembly"]
#![feature(plugin_registrar)]
#![allow(unstable)]
 
extern crate rustc;
extern crate syntax;
 
use std::collections::HashMap;
use syntax::ptr::P;
use syntax::ast;
use syntax::ast::{TokenTree, Expr};
use syntax::codemap;
use syntax::codemap::Pos;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use rustc::plugin::Registry; 
use syntax::parse::parser::Parser;
use syntax::parse::token;
use syntax::parse::token::{keywords, intern_and_get_ident};
use syntax::parse::common::seq_sep_trailing_allowed;
use syntax::ext::asm::expand_asm;
use syntax::print::pprust::token_to_string;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("asm", expand)
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
    dialect: syntax::ast::AsmDialect,
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
        base = "*".to_string() + base.as_slice();
    }

    let result = if input {
        base
    } else if c.early_clobber {
        "=&".to_string() + base.as_slice()
    } else {
        "=".to_string() + base.as_slice()
    };

    intern_and_get_ident(result.as_slice())
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

fn get_ident(p: &mut Parser) -> String {
    match p.token {
        token::Ident(id, _) => {
            p.bump();
            id.name.as_str().to_string()
        }
        _ => {
            p.unexpected()
        }
    }
}

fn parse_c(p: &mut Parser) -> Constraint {
    if p.eat(&token::BinOp(token::Percent)) {
        let early_clobber = p.eat(&token::BinOp(token::And));
        let indirect = p.eat(&token::BinOp(token::Star));
        Constraint { name: get_ident(p), indirect: indirect, early_clobber: early_clobber }
    } else {
        p.expect(&token::BinOp(token::Percent));
        unreachable!()
    }
}

fn parse_let(p: &mut Parser) -> String {
    p.expect_keyword(keywords::Let);
    let n = get_ident(p);
    p.expect(&token::Colon);
    n
}

fn parse_c_arrow(p: &mut Parser, data: &mut Data) -> usize {
    let c = parse_c(p);

    let rw = if p.eat(&token::Le) {
        p.expect(&token::Gt);
        true
    } else {
        p.expect(&token::FatArrow);
        false
    };

    let e = p.parse_expr();

    let kind = if rw {
        BindingKind::InputAndOutput(e)
    } else {
        BindingKind::Output(e)
    };

    add_binding(data, None, c, kind)
}

fn parse_operand(p: &mut Parser, data: &mut Data) -> usize {
    match p.token {
        token::BinOp(token::Percent) => {
            parse_c_arrow(p, data)
        }
        _ => {
            let lhs = p.parse_prefix_expr();
            let exp = p.parse_more_binops(lhs, syntax::ast_util::operator_prec(ast::BiBitOr));

            let rw = if p.eat(&token::Le) {
                p.expect(&token::Gt);
                true
            } else {
                p.expect(&token::FatArrow);
                false
            };

            let name = match p.token {
                token::Ident(_, _) => Some(parse_let(p)),
                _ => None
            };

            let c = parse_c(p);

            let kind = if rw {
                BindingKind::InputAndOutput(exp)
            } else if p.eat(&token::FatArrow) {
                BindingKind::InputThenOutput(exp, p.parse_expr())
            }  else {
                BindingKind::Input(exp)
            };

            add_binding(data, name, c, kind)
        }
    }
}

fn parse_binding(p: &mut Parser, data: &mut Data) -> usize {
    if p.token.is_keyword(keywords::Let) {
        let name = Some(parse_let(p));
        let c = parse_c(p);

        let kind = if p.eat(&token::FatArrow) {
            BindingKind::Output(p.parse_expr())
        } else {
            BindingKind::Bare
        };

        add_binding(data, name, c, kind)
    } else {
        parse_operand(p, data)
    }
}

fn parse_opt(cx: &mut ExtCtxt, p: &mut Parser, data: &mut Data) {
    if p.token.is_keyword(keywords::Use) {
        p.bump();
        data.clobbers.push(format!("~{{{}}}", get_ident(p).as_slice()));
    } else if p.token.is_keyword(keywords::Mod) {
        p.bump();
        match get_ident(p).as_slice() {
            "attsyntax" => {
                data.dialect = syntax::ast::AsmAtt;
            }
            "alignstack" => {
                data.alignstack = true;
            }
            "pure" => {
                data.volatile = false;
            }
            _ => cx.span_err(p.last_span, "unknown option")
        }
    } else {
        parse_binding(p, data);
    }
}

enum Output {
    Str(String),
    Binding(usize)
}

fn search<F: Fn(usize) -> bool>(fb: &codemap::FileMapAndBytePos, test: F, offset: usize) -> Option<u8> {
    let mut p = fb.pos.to_usize();

    loop {
        if test(p) {
            return None;
        }

        p = p + offset;

        let c = fb.fm.src.as_bytes()[p];

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
    let cm = &cx.parse_sess.span_diagnostic.cm;
    let fb = cm.lookup_byte_offset(sp.lo);
    is_whitespace(search(&fb, |p| { p == 0 }, -1))
}

fn is_whitespace_right(cx: &ExtCtxt, sp: codemap::Span) -> bool {
    let cm = &cx.parse_sess.span_diagnostic.cm;
    let mut fb = cm.lookup_byte_offset(sp.hi);
    fb.pos = codemap::Pos::from_usize(fb.pos.to_usize() - 1);
    is_whitespace(search(&fb, |p| { p >= fb.fm.src.len() - 1 }, 1))
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
            ast::TtToken(_, token::Literal(token::Lit::Str_(_), _)) | ast::TtToken(_, token::Literal(token::Lit::StrRaw(_, _), _)) => {
                return expand_asm(cx, sp, tts);
            }
            _ => ()
        }
    }

    let mut p = cx.new_parser_from_tts(tts);

    let mut data = Data { dialect: ast::AsmIntel, volatile: true, alignstack: false, clobbers: vec!(), idents: HashMap::new(), bindings: vec!() };

    match p.token {
        token::OpenDelim(token::Bracket) => {
            p.parse_unspanned_seq(&token::OpenDelim(token::Bracket),
                                  &token::CloseDelim(token::Bracket),
                                  seq_sep_trailing_allowed(token::Comma),
                                  |p| {
                                       parse_opt(cx, p, &mut data);
                                  });
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
                    out.push(Output::Binding(parse_binding(&mut p, &mut data)));

                    if is_whitespace_right(cx, p.span) {
                        out.push(Output::Str(" ".to_string()));
                    }
                    p.expect(&token::CloseDelim(token::Brace));
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
                p.unexpected();
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
                outputs.push((c_out, e, false));
                BindingIdx::Output(outputs.len())
            }
            BindingKind::InputThenOutput(e_in, e_out) => {
                let c_outpos = intern_and_get_ident(outputs.len().to_string().as_slice());
                inputs.push((c_outpos, e_in));
                outputs.push((c_out, e_out, false));
                BindingIdx::Input(inputs.len());
                BindingIdx::Output(outputs.len())
            }
            BindingKind::InputAndOutput(e) => {
                outputs.push((c_out, e, true));
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
            Output::Str(s) => out_str.push_str(s.as_slice()),
            Output::Binding(idx) => {
                out_str.push_str(" $");
                out_str.push_str(data.bindings[idx].idx.to_string().as_slice());
            }
        }
    }

    //println!("Clobbers: {}\nAssembly output: \n{}  \noutputs {}  \ninputs {}", data.clobbers, out_str, outputs, inputs);

    let expn_id = cx.codemap().record_expansion(codemap::ExpnInfo {
        call_site: sp,
        callee: codemap::NameAndSpan {
            name: "asm".to_string(),
            format: codemap::MacroBang,
            span: None,
        },
    });

    MacExpr::new(P(Expr {
        id: ast::DUMMY_NODE_ID,
        node: ast::ExprInlineAsm(ast::InlineAsm {
            asm: intern_and_get_ident(out_str.as_slice()),
            asm_str_style: ast::CookedStr,
            clobbers: data.clobbers.iter().map(|s| intern_and_get_ident(s.as_slice())).collect(),
            inputs: inputs,
            outputs: outputs,
            volatile: data.volatile,
            alignstack: data.alignstack,
            dialect: data.dialect,
            expn_id: expn_id,
        }),
        span: sp
    }))
}
