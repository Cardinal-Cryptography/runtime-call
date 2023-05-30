use std::io::Read;

use darling::{ast::NestedMeta, FromMeta};
use parity_scale_codec::Decode;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::proc_macro_error;
use quote::quote;
use subxt_metadata::Metadata;
use syn::parse_macro_input;

#[derive(Debug, FromMeta)]
struct MacroArgs {
    runtime_metadata_path: String,
}

impl MacroArgs {
    pub fn from_token_stream(input: TokenStream) -> darling::Result<Self> {
        let attr_args = match NestedMeta::parse_meta_list(input.into()) {
            Ok(v) => v,
            Err(e) => {
                return Err(darling::Error::from(e));
            }
        };
        Self::from_list(&attr_args)
    }
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn runtime_call(args: TokenStream, input: TokenStream) -> TokenStream {
    let item_mod = parse_macro_input!(input as syn::ItemMod);
    let macro_args = MacroArgs::from_token_stream(args).unwrap();

    let mut file = std::fs::File::open(&macro_args.runtime_metadata_path).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    let metadata = Metadata::decode(&mut &bytes[..]).unwrap();

    let mut main_enum_variants = vec![];
    let mut pallet_calls = vec![];

    for pallet in metadata.pallets() {
        let name = Ident::new(pallet.name(), Span::call_site());
        let index = pallet.index();
        let call_enum = Ident::new(&format!("{}Call", pallet.name()), Span::call_site());

        main_enum_variants.push(quote! {
            #[codec(index = #index)]
            #name ( #call_enum )
        });

        pallet_calls.push(quote! {
            #[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
            pub struct #call_enum;
        });
    }

    let mod_ident = item_mod.ident;

    (quote! {
        pub mod #mod_ident {
            #[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
            pub enum RuntimeCall {
                #(#main_enum_variants),*
            }

            #(#pallet_calls)*
        }
    })
    .into()
}
