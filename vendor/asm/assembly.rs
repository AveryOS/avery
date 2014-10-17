#![crate_type = "dylib"]
#![crate_name = "assembly"]
#![feature(plugin_registrar, if_let)]
 
extern crate rustc;
extern crate syntax;
extern crate debug;
 
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
use syntax::parse::common::seq_sep_trailing_disallowed;
use syntax::ext::asm::expand_asm;

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
    InputIdx(uint),
    OutputIdx(uint)
}

struct Constraint {
    name: String,
    early_clobber: bool
}

struct Binding {
    constraint: Constraint,
    kind: Option<BindingKind>,
    t_idx: BindingIdx,
    idx: uint
}

struct Data {
    dialect: syntax::ast::AsmDialect,
    alignstack: bool,
    volatile: bool,
    clobbers: Vec<String>,
    idents: HashMap<String, uint>,
    bindings: Vec<Binding>
}

fn format_c(c: &Constraint, input: bool) -> token::InternedString {
    let base = if c.name.len() > 1 {
        format!("{{{}}}", c.name)
    } else {
        c.name.clone()
    };

    let result = if input {
        base
    } else if c.early_clobber {
        "=&".to_string() + base
    } else {
        "=".to_string() + base
    };

    intern_and_get_ident(result.as_slice())
}

fn add_binding(data: &mut Data, name: Option<String>, c: Constraint, kind: BindingKind) -> uint {
    let idx = data.bindings.len();
    data.bindings.push(Binding {
        constraint: c,
        kind: Some(kind),
        t_idx: InputIdx(0),
        idx: 0
    });
    name.map(|name| {
        data.idents.insert(name, idx);
    });
    idx
}

fn get_ident(p: &mut Parser) -> String {
    match p.token {
        token::IDENT(id, _) => {
            p.bump();
            id.name.as_str().to_string()
        }
        _ => {
            p.unexpected()
        }
    }
}

fn parse_c(p: &mut Parser) -> Constraint {
    if p.eat(&token::BINOP(token::PERCENT)) {
        let early_clobber = p.eat(&token::BINOP(token::AND));
        Constraint { name: get_ident(p), early_clobber: early_clobber }
    } else {
        p.expect(&token::BINOP(token::PERCENT));
        unreachable!()
    }
}

fn parse_let(p: &mut Parser) -> String {
    p.expect_keyword(keywords::Let);
    let n = get_ident(p);
    p.expect(&token::COLON);
    n
}

fn parse_c_arrow(p: &mut Parser, data: &mut Data) -> uint {
    let c = parse_c(p);

    let rw = if p.eat(&token::LE) {
        p.expect(&token::GT);
        true
    } else {
        p.expect(&token::FAT_ARROW);
        false
    };

    let e = p.parse_expr();

    let kind = if rw {
        InputAndOutput(e)
    } else {
        Output(e)
    };

    add_binding(data, None, c, kind)
}

fn parse_operand(p: &mut Parser, data: &mut Data) -> uint {
    match p.token {
        token::BINOP(token::PERCENT) => {
            parse_c_arrow(p, data)
        }
        _ => {
            let lhs = p.parse_prefix_expr();
            let exp = p.parse_more_binops(lhs, syntax::ast_util::operator_prec(ast::BiBitOr));

            let rw = if p.eat(&token::LE) {
                p.expect(&token::GT);
                true
            } else {
                p.expect(&token::FAT_ARROW);
                false
            };

            let name = match p.token {
                token::IDENT(_, _) => Some(parse_let(p)),
                _ => None
            };

            let c = parse_c(p);

            let kind = if rw {
                InputAndOutput(exp)
            } else if p.eat(&token::FAT_ARROW) {
                InputThenOutput(exp, p.parse_expr())
            }  else {
                Input(exp)
            };

            add_binding(data, name, c, kind)
        }
    }
}

fn parse_binding(p: &mut Parser, data: &mut Data) -> uint {
    if p.is_keyword(keywords::Let) {
        let name = Some(parse_let(p));
        let c = parse_c(p);

        let kind = if p.eat(&token::FAT_ARROW) {
            Output(p.parse_expr())
        } else {
            Bare
        };

        add_binding(data, name, c, kind)
    } else {
        parse_operand(p, data)
    }
}

fn parse_opt(cx: &mut ExtCtxt, p: &mut Parser, data: &mut Data) {
    if p.is_keyword(keywords::Use) {
        p.bump();
        data.clobbers.push(format!("~{{{}}}", get_ident(p).as_slice()));
    } else if p.is_keyword(keywords::Mod) {
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

enum OutputType {
    OutputStr(String),
    OutputBinding(uint)
}

fn search(fb: &codemap::FileMapAndBytePos, test: |pos: uint| -> bool, offset: uint) -> Option<u8> {
    let mut p = fb.pos.to_uint();

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
    let fb = cm.lookup_byte_offset(sp.hi);
    is_whitespace(search(&fb, |p| { p >= fb.fm.src.len() - 1 }, 1))
}

fn expand<'cx>(cx: &'cx mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree]) -> Box<MacResult + 'cx> {
    // Fall back to the old syntax if we start with a string
    if tts.len() > 0 {
        match tts[0] {
            ast::TTTok(_, token::LIT_STR(_)) | ast::TTTok(_, token::LIT_STR_RAW(_, _)) => {
                return expand_asm(cx, sp, tts);
            }
            _ => ()
        }
    }

    let mut p = cx.new_parser_from_tts(tts);

    let mut data = Data { dialect: ast::AsmIntel, volatile: true, alignstack: false, clobbers: vec!(), idents: HashMap::new(), bindings: vec!() };

    match p.token {
        token::LBRACKET => {
            p.parse_unspanned_seq(&token::LBRACKET,
                                  &token::RBRACKET,
                                  seq_sep_trailing_disallowed(token::COMMA),
                                  |p| {
                                       parse_opt(cx, p, &mut data);
                                  });
        }
        _ => ()
    }

    let mut out = vec!(OutputStr("\t".to_string()));

    let whitespace_wrap = |out: &mut Vec<OutputType>, sp, act: |&mut Vec<OutputType>| -> ()| {
        if is_whitespace_left(cx, sp) {
            out.push(OutputStr(" ".to_string()));
        }
        act(out);
        if is_whitespace_right(cx, sp) {
            out.push(OutputStr(" ".to_string()));
        }
    };

    loop {
        //println!("Token!{:?}!", p.token);
    
        match p.token {
            token::LBRACE => {
                    if is_whitespace_left(cx, p.span) {
                        out.push(OutputStr(" ".to_string()));
                    }

                    p.bump();
                    out.push(OutputBinding(parse_binding(&mut p, &mut data)));

                    if is_whitespace_right(cx, p.span) {
                        out.push(OutputStr(" ".to_string()));
                    }
                    p.expect(&token::RBRACE);
            }
            token::IDENT(_, _) => {
                whitespace_wrap(&mut out, p.span, |out| {
                    if let Some(idx) = data.idents.find(&token::to_string(&p.token)) {
                        out.push(OutputBinding(*idx));
                        p.bump();
                    } else {
                        out.push(OutputStr(token::to_string(&p.token)));
                        p.bump();
                    }
                });
            }
            token::DOLLAR => {
                whitespace_wrap(&mut out, p.span, |out| {
                    out.push(OutputStr("$$".to_string()));
                });
                p.bump();
            }
            token::SEMI => {
                out.push(OutputStr("\n\t".to_string()));
                p.bump();
            }
            token::INTERPOLATED(_) => {
                p.unexpected();
            }
            token::EOF => break,
            _ => {
                whitespace_wrap(&mut out, p.span, |out| {
                    out.push(OutputStr(token::to_string(&p.token)));
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
            Bare => {
                fail!("Bare unsupported")
                //let c_clobber = intern_and_get_ident(("=&".to_string() + c).as_slice());
                // this needs an expression - outputs.push((c_clobber, , false));
                OutputIdx(outputs.len())
            }
            Input(e) => {
                inputs.push((c_in, e));
                InputIdx(inputs.len())
            }
            Output(e) => {
                outputs.push((c_out, e, false));
                OutputIdx(outputs.len())
            }
            InputThenOutput(e_in, e_out) => {
                let c_outpos = intern_and_get_ident(outputs.len().to_string().as_slice());
                inputs.push((c_outpos, e_in));
                outputs.push((c_out, e_out, false));
                InputIdx(inputs.len());
                OutputIdx(outputs.len())
            }
            InputAndOutput(e) => {
                outputs.push((c_out, e, true));
                OutputIdx(outputs.len())
            }
        }
    }

    for b in data.bindings.iter_mut() {
        b.idx = match b.t_idx {
            InputIdx(i) => outputs.len() + i,
            OutputIdx(i) => i
        } - 1;
    }

    let mut out_str = "".to_string();

    for o in out.into_iter() {
        match o {
            OutputStr(s) => out_str.push_str(s.as_slice()),
            OutputBinding(idx) => {
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
            clobbers: intern_and_get_ident(data.clobbers.connect(",").as_slice()),
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
