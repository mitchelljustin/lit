extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote};
use syn::{Data, parse_macro_input, Type};
use syn::spanned::Spanned;

use lit::model::SqliteColumnType;

#[proc_macro_error]
#[proc_macro_derive(Model)]
pub fn my_derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = parse_macro_input!(input);
    let model_name = input.ident.clone();
    let Data::Struct(model_struct) = input.data else {
        abort!(input.span(), "Model can only be derived from struct");
    };
    let model_name_string = model_name.to_string();
    let fields = model_struct.fields.iter().filter_map(|field| {
        let name = field.ident.clone()?;
        let name = name.to_string();
        let sqlite_type = match &field.ty {
            Type::Path(path) => {
                let first_segment = path.path.segments.first().unwrap();
                match first_segment.ident.to_string().as_str() {
                    "String" => SqliteColumnType::TEXT,
                    "i64" | "u64" => SqliteColumnType::INTEGER,
                    "f64" => SqliteColumnType::REAL,
                    "bool" => SqliteColumnType::INTEGER,
                    _ => abort!(
                        field.ty.span(),
                        "only allowed types for fields are: String, i64, u64, f64, bool"
                    ),
                }
            }
            _ => abort!(
                field.ty.span(),
                "only allowed types for fields are: String, i64, u64, f64, bool"
            ),
        };
        let sqlite_type = format_ident!("{sqlite_type}");
        Some(quote! {
            lit::model::ModelField {
                name: #name,
                col_type: lit::model::SqliteColumnType::#sqlite_type,
                _marker: std::marker::PhantomData,
            }
        })
    });
    let tokens = quote! {
        impl lit::model::ModelStruct for #model_name {
            fn model_name() -> &'static str {
                #model_name_string
            }

            fn fields() -> lit::model::ModelFields<Self> {
                lit::model::ModelFields(
                    vec![
                        #(#fields),*
                    ],
                )
            }
        }
    };

    tokens.into()
}
