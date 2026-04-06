use either::Either::{self, Left, Right};
use proc_macro2::{Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Expr, ExprBlock, ExprCall, ExprMethodCall, Ident, Pat, Path, Token, braced, parenthesized,
    parse::{Parse, ParseBuffer, discouraged::Speculative},
    parse_quote, parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    token,
};

mod kw {
    syn::custom_keyword!(theme);
    syn::custom_keyword!(INIT);
    syn::custom_keyword!(UPDATE);
    syn::custom_keyword!(ON);
    syn::custom_keyword!(KEY);
    syn::custom_keyword!(BIND);
    syn::custom_keyword!(BODY);
    syn::custom_keyword!(ARGS);
    syn::custom_keyword!(STATE);
}

pub struct Block {
    elems: Vec<BlockElem>,
}
pub enum BlockElem {
    Node {
        name: Ident,
        body: Block,
    },
    Init(Punctuated<Assignment, Token![,]>),
    Update(Punctuated<Assignment, Token![,]>),
    On(Punctuated<Assignment, Token![,]>),
    Code(TokenStream),
    Call(ExprCall, Option<Punctuated<Pat, Token![,]>>, Block),
    MethodCall(ExprMethodCall, Option<Punctuated<Pat, Token![,]>>, Block),
    If(IfElem),
    For {
        pat: Pat,
        iter: Expr,
        key: Expr,
        body: Block,
    },
    Match {
        value: Expr,
        arms: Vec<(Pat, Block)>,
    },
    Body(Punctuated<Expr, Token![,]>),
    State(Ident, TokenStream),
    // Bind(Ident),
}
pub struct IfElem {
    cond: Expr,
    body: Block,
    else_expr: Option<Either<Box<IfElem>, Block>>,
}
pub struct Assignment {
    theme: Option<Ident>,
    name: Ident,
    value: Expr,
}

impl Parse for IfElem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![if]>()?;
        let cond = Expr::parse_without_eager_brace(input)?;
        let inner;
        braced!(inner in input);
        let body = inner.parse()?;
        let else_expr = if input.peek(Token![else]) {
            input.parse::<Token![else]>()?;
            Some(if input.peek(Token![if]) {
                Left(Box::new(input.parse()?))
            } else {
                let inner;
                braced!(inner in input);
                Right(inner.parse()?)
            })
        } else {
            None
        };

        Ok(Self {
            cond,
            body,
            else_expr,
        })
    }
}

impl Parse for Assignment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name;
        let theme = if input.peek(kw::theme) && input.peek2(token::Paren) {
            input.parse::<kw::theme>()?;
            let inner;
            parenthesized!(inner in input);
            let typ = inner.parse::<Ident>()?;
            inner.parse::<Token![,]>()?;
            name = inner.parse::<Ident>()?;
            Some(typ)
        } else {
            name = input.parse::<Ident>()?;
            None
        };
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Assignment { theme, name, value })
    }
}

impl Parse for BlockElem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) && input.peek2(Token![..]) {
            let name = input.parse::<Ident>()?;
            input.parse::<Token![..]>()?;
            let inner;
            braced!(inner in input);
            let elem = BlockElem::Node {
                name,
                body: inner.parse()?,
            };
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::INIT) {
            input.parse::<kw::INIT>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::Init(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::UPDATE) {
            input.parse::<kw::UPDATE>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::Update(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::ON) {
            input.parse::<kw::ON>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::On(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::STATE) {
            input.parse::<kw::STATE>()?;
            let inner;
            parenthesized!(inner in input);
            let name = inner.parse()?;
            inner.parse::<Token![=]>()?;
            let value = inner.parse()?;
            let elem = BlockElem::State(name, value);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        }
        // else if input.peek(kw::BIND) {
        //     input.parse::<kw::BIND>()?;
        //     let inner;
        //     parenthesized!(inner in input);
        //     let var = inner.parse()?;
        //     input.parse::<Token![;]>()?;
        //     Ok(BlockElem::Bind(var))
        //     //
        // }
        else if input.peek(kw::BODY) {
            input.parse::<kw::BODY>()?;
            let inner;
            parenthesized!(inner in input);
            let args = Punctuated::parse_terminated(&inner)?;
            input.parse::<Token![;]>()?;
            Ok(BlockElem::Body(args))
            //
        } else if input.peek(token::Brace) {
            let inner;
            braced!(inner in input);
            Ok(BlockElem::Code(inner.parse()?))
        } else if input.peek(Token![if]) {
            let elem = BlockElem::If(input.parse()?);
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            }
            Ok(elem)
        } else if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let pat = Pat::parse_single(input)?;
            input.parse::<Token![in]>()?;
            let iter = Expr::parse_without_eager_brace(input)?;

            let inner;
            braced!(inner in input);
            inner.parse::<kw::KEY>()?;
            let inner2;
            parenthesized!(inner2 in inner);
            let key = inner2.parse()?;
            inner.parse::<Token![;]>()?;
            let body = inner.parse()?;

            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            }
            Ok(BlockElem::For {
                pat,
                iter,
                key,
                body,
            })
        } else if input.peek(Token![match]) {
            input.parse::<Token![match]>()?;
            let value = Expr::parse_without_eager_brace(input)?;

            let inner;
            braced!(inner in input);
            let mut arms = vec![];
            while !inner.is_empty() {
                let pat = Pat::parse_multi_with_leading_vert(&inner)?;
                inner.parse::<Token![=>]>()?;

                let inner2;
                braced!(inner2 in inner);
                let block = inner2.parse()?;
                if inner.peek(Token![,]) {
                    inner.parse::<Token![,]>()?;
                }
                arms.push((pat, block));
            }

            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            }
            Ok(BlockElem::Match { value, arms })
        } else {
            let mut toks = vec![];
            input.step(|cursor| {
                let mut rest = *cursor;
                let mut found_first = false;
                while let Some((tt, next)) = rest.token_tree() {
                    toks.push(tt.clone());
                    match &tt {
                        TokenTree::Punct(punct) if punct.as_char() == '.' => {
                            if found_first {
                                toks.pop();
                                toks.pop();
                                return Ok(((), next));
                            }
                            if punct.spacing() == Spacing::Joint {
                                found_first = true
                            }
                        }
                        _ => {
                            found_first = false;
                        }
                    }
                    rest = next
                }
                Err(cursor.error("bad0"))
            })?;

            let mut left = TokenStream::new();
            left.extend(toks);
            let expr = parse2::<Expr>(left)?;

            let parse_args =
                |binput: &ParseBuffer| -> Result<Option<Punctuated<Pat, Token![,]>>, syn::Error> {
                    Ok(if binput.peek(Token![let]) {
                        binput.parse::<Token![let]>()?;
                        let inner2;
                        parenthesized!(inner2 in binput);
                        let args =
                            Punctuated::parse_terminated_with(&inner2, |i| Pat::parse_single(i))?;
                        binput.parse::<Token![=]>()?;
                        binput.parse::<kw::ARGS>()?;
                        binput.parse::<Token![;]>()?;
                        Some(args)
                    } else {
                        None
                    })
                };
            let elem = match expr {
                Expr::Call(mut expr_call) => {
                    expr_call.args.insert(
                        0,
                        parse_quote_spanned!(expr_call.func.span() => __builder.upcast()),
                    );
                    let inner;
                    braced!(inner in input);
                    let args = parse_args(&inner)?;
                    let block = inner.parse()?;

                    BlockElem::Call(expr_call, args, block)
                }
                Expr::MethodCall(mut expr_method_call) => {
                    expr_method_call.args.insert(
                        0,
                        parse_quote_spanned!(expr_method_call.receiver.span() => __builder.upcast()),
                    );
                    let inner;
                    braced!(inner in input);
                    let args = parse_args(&inner)?;
                    let block = inner.parse()?;

                    BlockElem::MethodCall(expr_method_call, args, block)
                }
                _ => panic!("bad1"),
            };

            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        }
    }
}
impl Parse for Block {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut elems = vec![];
        while !input.is_empty() {
            elems.push(input.parse()?);
        }
        Ok(Block { elems })
    }
}

pub fn gen_block(block: Block) -> TokenStream {
    let mut out = quote! {};
    for i in block.elems {
        let gen_init_or_update = |i: Assignment| -> TokenStream {
            let name = i.name;
            let value = i.value;
            if let Some(theme) = i.theme {
                let func = format_ident!("__set_theme_{}_override", theme);
                quote! {
                    __builder = __builder.#func(stringify!(#name), #value);
                }
            } else {
                quote! {
                    __builder = __builder.__set_prop(stringify!(#name), #value);
                }
            }
        };
        match i {
            BlockElem::Node { name, body } => {
                let body = gen_block(body);
                out.extend(quote! {
                    __builder = __builder.__child::<#name>(|mut __builder| {
                        #body
                    });
                });
            }
            BlockElem::Init(list) => {
                let mut inner = quote! {};
                for i in list {
                    inner.extend(gen_init_or_update(i));
                }
                out.extend(quote! {
                    if __builder.init() {
                        #inner
                    }
                });
            }
            BlockElem::Update(list) => {
                for i in list {
                    out.extend(gen_init_or_update(i));
                }
            }
            BlockElem::On(list) => {
                for i in list {
                    if i.theme.is_some() {
                        panic!("bad2");
                    }
                    let name = i.name;
                    let value = i.value;
                    out.extend(quote! {
                        __builder = __builder.__signal(stringify!(#name), #value);
                    });
                }
            }
            BlockElem::State(name, init) => {
                out.extend(quote! {
                    (__builder, #name) = __builder.__state(#init);
                });
            }
            BlockElem::Code(block) => {
                out.extend(quote! { #block });
            }
            BlockElem::Call(mut expr_call, args, block) => {
                let block = gen_block(block);
                let args = args
                    .map(|mut args| {
                        if !args.empty_or_trailing() {
                            args.push_punct(parse_quote! {,});
                        };
                        quote! { (#args) }
                    })
                    .unwrap_or_else(|| quote! {_});
                expr_call.args.push(
                    parse_quote_spanned!(expr_call.func.span() => &mut |mut __builder, #args| {
                        #block
                    }),
                );
                out.extend(quote! {
                    __builder = (#expr_call).cast();
                });
            }
            BlockElem::MethodCall(mut expr_method_call, args, block) => {
                let block = gen_block(block);
                let args = args
                    .map(|mut args| {
                        if !args.empty_or_trailing() {
                            args.push_punct(parse_quote! {,});
                        };
                        quote! { (#args) }
                    })
                    .unwrap_or_else(|| quote! {_});
                expr_method_call.args.push(
                    parse_quote_spanned!(expr_method_call.receiver.span() => &mut |mut __builder, #args| {
                        #block
                    }),
                );
                out.extend(quote! {
                    __builder = (#expr_method_call).cast();
                });
            }
            BlockElem::If(if_elem) => {
                let mut current = Some(if_elem);
                let mut idx = 0u64;
                out.extend(quote! {
                    __builder =
                });
                while let Some(if_elem) = current.take() {
                    let cond = if_elem.cond;
                    let body = gen_block(if_elem.body);
                    out.extend(quote! {
                        if #cond {
                            __builder.__under_explicit(#idx, |mut __builder| {
                                #body
                            })
                        }
                    });
                    match if_elem.else_expr {
                        Some(Left(next)) => {
                            idx += 1;
                            out.extend(quote! { else });
                            current = Some(*next);
                        }
                        Some(Right(remaining)) => {
                            idx += 1;
                            let body = gen_block(remaining);
                            out.extend(quote! { else {
                                __builder.__under_explicit(#idx, |mut __builder| {
                                    #body
                                })
                            } });
                        }
                        None => {
                            out.extend(quote! { else { __builder } });
                        }
                    }
                }
                out.extend(quote! {
                    ;
                });
            }
            BlockElem::For {
                pat,
                iter,
                key,
                body,
            } => {
                let body = gen_block(body);
                out.extend(quote! {
                    for #pat in #iter {
                        __builder = __builder.__under_explicit(#key, |mut __builder| {
                            #body
                        });
                    }
                });
                //
            }
            BlockElem::Body(mut args) => {
                if !args.empty_or_trailing() {
                    args.push_punct(parse_quote! {,});
                }
                out.extend(quote! {
                    __builder = __body(__builder.upcast(), (#args)).cast();
                });
            }
            BlockElem::Match { value, arms } => {
                let mut inner = quote! {};
                for (idx, (pat, block)) in arms.into_iter().enumerate() {
                    let body = gen_block(block);
                    let idx = idx as u64;
                    inner.extend(quote! {
                        #pat => {
                            __builder.__under_explicit(#idx, |mut __builder| {
                                #body
                            })
                        },
                    });
                }
                out.extend(quote! {
                    __builder = match #value {
                        #inner
                    };
                });
            }
        }
    }
    out.extend(quote! { __builder });
    out
}
