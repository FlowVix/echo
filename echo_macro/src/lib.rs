use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, FnArg, Ident, PatIdent, PatType, Path, Result, Signature, Token, Type, Visibility,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
};

use crate::tree::{Block, gen_block};

mod tree;

struct RawFn {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    body: TokenStream,
}

impl Parse for RawFn {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;

        let body;
        syn::braced!(body in input);
        let body: TokenStream = body.parse()?;

        Ok(RawFn {
            attrs,
            vis,
            sig,
            body,
        })
    }
}

struct TreeAttr {
    btype: Path,
    args: Punctuated<Type, Token![,]>,
    usability: Option<Path>,
}
impl Parse for TreeAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let btype = input.parse()?;
        let inner;
        parenthesized!(inner in input);
        let args = Punctuated::parse_terminated(&inner)?;
        let usability = input
            .peek(Token![in])
            .then(|| {
                input.parse::<Token![in]>()?;
                input.parse()
            })
            .transpose()?;
        Ok(TreeAttr {
            btype,
            args,
            usability,
        })
    }
}

#[proc_macro_attribute]
pub fn tree(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let TreeAttr {
        btype,
        mut args,
        usability,
    } = parse_macro_input!(attr as TreeAttr);
    if !args.empty_or_trailing() {
        args.push_punct(parse_quote! {,});
    }
    let mut func = parse_macro_input!(item as RawFn);

    let has_self = !func.sig.inputs.is_empty() && matches!(func.sig.inputs[0], FnArg::Receiver(_));
    let usability = usability
        .map(|v| quote! { #v })
        .unwrap_or(quote! { ::godot::classes::Node });
    func.sig.inputs.insert(
        if has_self { 1 } else { 0 },
        FnArg::Typed(PatType {
            attrs: vec![],
            pat: parse_quote!(mut __builder),
            colon_token: parse_quote!(:),
            ty: parse_quote!(::echo::Builder<#usability>),
        }),
    );
    func.sig.inputs.push(FnArg::Typed(PatType {
        attrs: vec![],
        pat: parse_quote!(__body),
        colon_token: parse_quote!(:),
        ty: parse_quote!(&mut dyn FnMut(::echo::Builder<#btype>, (#args)) -> ::echo::Builder<#btype>),
    }));
    func.sig.output = parse_quote!(-> ::echo::Builder<#usability>);

    let attrs = func.attrs;
    let vis = func.vis;
    let sig = func.sig;
    let body = func.body;

    let block: Block = syn::parse2::<Block>(body).unwrap();
    let block = gen_block(block);

    // body is still raw tokens here
    let out: TokenStream = quote! {
        #(#attrs)*
        #vis #sig {
            #block
        }
    };
    out.into()
}
