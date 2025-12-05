use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, Token, LitStr, Expr, Result, punctuated::Punctuated};

struct ContextMenuItem {
    label: LitStr,
    _arrow: Token![=>],
    action: Expr,
}

impl Parse for ContextMenuItem {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ContextMenuItem {
            label: input.parse()?,
            _arrow: input.parse()?,
            action: input.parse()?,
        })
    }
}

struct ContextMenuInput {
    items: Punctuated<ContextMenuItem, Token![,]>,
}

impl Parse for ContextMenuInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ContextMenuInput {
            items: input.parse_terminated(ContextMenuItem::parse, Token![,])?,
        })
    }
}

pub fn context_menu(input: TokenStream) -> TokenStream {
    let menu_input: ContextMenuInput = match syn::parse2(input) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };

    let items = menu_input.items.iter().map(|item| {
        let label = &item.label;
        let action = &item.action;
        quote! {
            nptk_core::menu::ContextMenuItem::Action {
                label: #label.to_string(),
                action: std::sync::Arc::new(move || { #action; }),
            }
        }
    });

    quote! {
        nptk_core::menu::ContextMenu {
            items: vec![
                #(#items),*
            ]
        }
    }
}
