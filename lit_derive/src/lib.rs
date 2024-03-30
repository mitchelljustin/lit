#![feature(let_chains)]
extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::Ident;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote};
use syn::{Data, Field, Fields, parse_macro_input, Type};
use syn::spanned::Spanned;

use lit::model::SqliteColumnType;

#[derive(Clone)]
struct ModelFieldMeta {
    name: Ident,
    col_type: SqliteColumnType,
    field: Field,
}

#[proc_macro_error]
#[proc_macro_derive(ModelStruct)]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = parse_macro_input!(input);
    let model_name = input.ident.clone();
    let Data::Struct(model_struct) = input.data else {
        abort!(input.span(), "Model can only be derived from struct");
    };
    let model_name_string = model_name.to_string();
    let Fields::Named(named_fields) = model_struct.fields else {
        abort!(model_struct.fields.span(), "model fields must be named",);
    };
    let mut fields_iter = named_fields.named.iter();
    let Some(id_field) = fields_iter.next() else {
        abort!(named_fields.span(), "model struct needs to have fields",);
    };
    if id_field.ident.as_ref().unwrap() != "id" {
        abort!(id_field.span(), "first model field must be named `id`",);
    };
    let Type::Path(path) = &id_field.ty else {
        abort!(id_field.ty.span(), "`id` field must be type `i64`");
    };
    if path.path.segments.first().unwrap().ident != "i64" {
        abort!(id_field.ty.span(), "`id` field must be type `i64`");
    }
    let model_fields: Vec<_> = fields_iter
        .map(|field| {
            let name = field.ident.clone().unwrap();
            let col_type = column_type_of(field);
            ModelFieldMeta {
                name,
                col_type,
                field: field.clone(),
            }
        })
        .collect();
    let field_names = model_fields.iter().map(|f| f.name.clone());
    let fields_quoted =
        model_fields
            .iter()
            .cloned()
            .map(|ModelFieldMeta { name, col_type, .. }| {
                let col_type = format_ident!("{col_type}");
                let name = name.to_string();
                quote! {
                    lit::model::ModelField {
                        name: #name,
                        col_type: lit::model::SqliteColumnType::#col_type,
                        _marker: std::marker::PhantomData,
                    }
                }
            });
    let values_quoted =
        model_fields
            .iter()
            .cloned()
            .map(|ModelFieldMeta { name, col_type, .. }| {
                let col_type = format_ident!("{col_type}");
                quote! {
                    lit::model::SqliteValue::#col_type(self.#name.clone().into())
                }
            });
    let model_method_sigs = model_fields
        .iter()
        .cloned()
        .map(|ModelFieldMeta { name, field, .. }| {
            let method_name = format_ident!("find_by_{name}");
            let arg_type = field.ty;
            quote! {
                fn #method_name(&self, value: impl Into<#arg_type>) -> lit::Result<std::vec::Vec<#model_name>>
            }
        })
        .zip(model_fields.iter().cloned())
        .collect::<Vec<_>>();
    let model_trait_methods = model_method_sigs.iter().cloned().map(|(fn_sig, _)| {
        quote! {
            #fn_sig;
        }
    });
    let model_trait_method_impls =
        model_method_sigs
            .iter()
            .cloned()
            .map(|(fn_sig, ModelFieldMeta { name, .. })| {
                let selector = format!("{name}=?");
                quote! {
                    #fn_sig {
                        let param = lit::model::SqliteValue::from(value.into());
                        self.select(
                            #selector,
                            (param,),
                        )
                    }
                }
            });
    let trait_name = format_ident!("{model_name}Methods");
    let tokens = quote! {
        impl lit::model::Model for #model_name {
            fn id(&self) -> Option<i64> {
                if self.id == 0 {
                    None
                } else {
                    Some(self.id)
                }
            }

            fn model_name() -> &'static str {
                #model_name_string
            }

            fn fields() -> lit::model::ModelFields<Self> {
                lit::model::ModelFields(
                    vec![
                        #(#fields_quoted),*
                    ],
                )
            }

            fn as_params(&self) -> std::vec::Vec<lit::model::SqliteValue> {
                vec![
                    lit::model::SqliteValue::INTEGER(self.id),
                    #(#values_quoted),*
                ]
            }

            fn from_row(row: impl IntoIterator<Item=lit::model::SqliteValue>) -> Option<Self> {
                let mut iter = row.into_iter();
                Some(Self {
                    id: <Option<i64>>::from(iter.next()?)?,
                    #(#field_names:  <Option<_>>::from(iter.next()?)?),*
                })
            }
        }

        trait #trait_name {
            #(#model_trait_methods)*
        }

        impl #trait_name for lit::query_set::QuerySet<#model_name> {
            #(#model_trait_method_impls)*
        }
    };

    tokens.into()
}

fn column_type_of(field: &Field) -> SqliteColumnType {
    match &field.ty {
        Type::Path(path) => {
            let first_segment = path.path.segments.first().unwrap();
            match first_segment.ident.to_string().as_str() {
                "String" => SqliteColumnType::TEXT,
                "i64" => SqliteColumnType::INTEGER,
                "f64" => SqliteColumnType::REAL,
                "bool" => SqliteColumnType::INTEGER,
                _ => abort!(
                    field.ty.span(),
                    "only allowed types for fields are: String, i64, f64, bool"
                ),
            }
        }
        _ => abort!(
            field.ty.span(),
            "only allowed types for fields are: String, i64, f64, bool"
        ),
    }
}
