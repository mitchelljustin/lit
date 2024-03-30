extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data};

#[proc_macro_error]
#[proc_macro_derive(Model)]
pub fn my_derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = parse_macro_input!(input);
    let model_name = input.ident.clone();
    let Data::Struct(model_struct) = input.data else {
        proc_macro_error::abort!(input.span(), "Model can only be derived from struct");
    };
    let model_name_string = model_name.to_string();
    
    let tokens = quote! {
        impl lit::model::ModelStruct for #model_name {
            fn model_name() -> &'static str {
                #model_name_string
            }
        }
    };

    tokens.into()
}
