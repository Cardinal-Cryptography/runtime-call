use std::io::Read;

use darling::{ast::NestedMeta, FromMeta};
use parity_scale_codec::Decode;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_error::proc_macro_error;
use quote::quote;
use subxt_metadata::Metadata;
use syn::{parse_macro_input, Path};

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

    let types_registry = &metadata.types();

    let mut pallets = metadata.pallets().collect::<Vec<_>>();
    pallets.sort_by_key(|p| p.index());

    let mut main_enum_variants = vec![];
    let mut pallet_calls = vec![];

    for pallet in pallets {
        let pallet_name = Ident::new(pallet.name(), Span::call_site());
        let pallet_index = pallet.index();
        let pallet_call_name = Ident::new(&format!("{}Call", pallet.name()), Span::call_site());

        main_enum_variants.push(quote! {
            #[codec(index = #pallet_index)]
            #pallet_name ( #pallet_call_name )
        });

        let mut call_variants = vec![];

        for call_variant in pallet.call_variants().unwrap_or_default() {
            let call_index = call_variant.index;
            let call_name = Ident::new(&call_variant.name, Span::call_site());

            let mut fields = vec![];
            for field in &call_variant.fields {
                let field_name = Ident::new(field.name.as_ref().unwrap(), Span::call_site());

                let field_type = types_registry.resolve(field.ty.id).unwrap();
                let field_type = match field_type.path.ident() {
                    Some(ident) => Path::from(Ident::new(&ident, Span::call_site())),
                    None => Path::from(Ident::new("u8", Span::call_site())),
                };

                fields.push(quote! {#field_name: #field_type});
            }

            call_variants.push(quote! {
                #[codec(index = #call_index)]
                #call_name {
                    #(#fields),*
                }
            });
        }

        pallet_calls.push(quote! {
            #[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
            pub enum #pallet_call_name {
                #(#call_variants),*
            }
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
