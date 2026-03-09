use either::Either::{self, Left, Right};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprCall, ExprMethodCall, Ident, Pat, Token, braced, parenthesized, parse::Parse,
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token,
};

mod kw {
    syn::custom_keyword!(theme);
    syn::custom_keyword!(INIT);
    syn::custom_keyword!(UPDATE);
    syn::custom_keyword!(ON);
    syn::custom_keyword!(KEY);
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
    Call(ExprCall),
    MethodCall(ExprMethodCall),
    If(IfElem),
    For {
        pat: Pat,
        iter: Expr,
        key: Expr,
        body: Block,
    },
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
        let theme = if input.peek(kw::theme) {
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
            input.parse::<Ident>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::Init(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::UPDATE) {
            input.parse::<Ident>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::Update(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
            //
        } else if input.peek(kw::ON) {
            input.parse::<Ident>()?;
            let inner;
            parenthesized!(inner in input);
            let elem = BlockElem::On(Punctuated::parse_terminated(&inner)?);
            input.parse::<Token![;]>()?;
            Ok(elem)
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
        } else {
            let expr = input.parse::<Expr>()?;
            let elem = match expr {
                Expr::Call(mut expr_call) => {
                    expr_call.args.insert(
                        0,
                        parse_quote_spanned!(expr_call.func.span() => __builder.__upcast()),
                    );
                    BlockElem::Call(expr_call)
                }
                Expr::MethodCall(mut expr_method_call) => {
                    expr_method_call.args.insert(
                        0,
                        parse_quote_spanned!(expr_method_call.receiver.span() => __builder.__upcast()),
                    );
                    BlockElem::MethodCall(expr_method_call)
                }
                _ => panic!("bad"),
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
        let gen_init_or_update = |i: Assignment, update: bool| -> TokenStream {
            let name = i.name;
            let value = i.value;
            if let Some(theme) = i.theme {
                let func = format_ident!("__set_{}_override", theme);
                quote! {
                    let __builder = __builder.#func(stringify!(#name), #value, #update);
                }
            } else {
                quote! {
                    let __builder = __builder.__set_prop(stringify!(#name), #value, #update);
                }
            }
        };
        match i {
            BlockElem::Node { name, body } => {
                let body = gen_block(body);
                out.extend(quote! {
                    let __builder = __builder.__child::<#name>(|__builder| {
                        #body
                    });
                });
            }
            BlockElem::Init(list) => {
                for i in list {
                    out.extend(gen_init_or_update(i, false));
                }
            }
            BlockElem::Update(list) => {
                for i in list {
                    out.extend(gen_init_or_update(i, true));
                }
            }
            BlockElem::On(list) => {
                for i in list {
                    if i.theme.is_some() {
                        panic!("bad");
                    }
                    let name = i.name;
                    let value = i.value;
                    out.extend(quote! {
                        let __builder = __builder.__signal(stringify!(#name), #value);
                    });
                }
            }
            BlockElem::Code(block) => {
                out.extend(quote! { #block });
            }
            BlockElem::Call(expr_call) => {
                out.extend(quote! {
                    let __builder = (#expr_call).__cast();
                });
            }
            BlockElem::MethodCall(expr_method_call) => {
                out.extend(quote! {
                    let __builder = (#expr_method_call).__cast();
                });
            }
            BlockElem::If(if_elem) => {
                let mut current = Some(if_elem);
                let mut idx = 0u64;
                out.extend(quote! {
                    let __builder =
                });
                while let Some(if_elem) = current.take() {
                    let cond = if_elem.cond;
                    let body = gen_block(if_elem.body);
                    out.extend(quote! {
                        if #cond {
                            __builder.__under_explicit(#idx, |__builder| {
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
                                __builder.__under_explicit(#idx, |__builder| {
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
                    let mut __builder = __builder;

                    for #pat in #iter {
                        __builder = __builder.__under_explicit(#key, |__builder| {
                            #body
                        });
                    }
                });
                //
            }
        }
    }
    out.extend(quote! { __builder });
    out
}
