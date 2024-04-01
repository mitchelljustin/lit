#![feature(let_chains)]
extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::Ident;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote};
use syn::{Data, Field, Fields, Meta, parse_macro_input, Path, Type};
use syn::spanned::Spanned;

#[derive(Clone)]
struct ModelFieldMeta {
    name: Ident,
    col_type: rusqlite::types::Type,
    field: Field,
    foreign_key_model: Option<Path>,
}

#[proc_macro_error]
#[proc_macro_derive(ModelStruct, attributes(foreign_key))]
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
            let foreign_key = field
                .attrs
                .iter()
                .filter_map(|attr| {
                    let Meta::List(list) = &attr.meta else {
                        return None;
                    };
                    if list.path.segments.first().unwrap().ident != "foreign_key" {
                        return None;
                    }
                    let Ok(linked_model) = syn::parse2::<Path>(list.tokens.clone()) else {
                        abort!(list.tokens.span(), "foreign key target must be a path");
                    };
                    Some(linked_model)
                })
                .next();
            if foreign_key.is_some() && !name.to_string().ends_with("_id") {
                abort!(name.span(), "name of foreign key field must end with '_id'",);
            }
            ModelFieldMeta {
                name,
                col_type,
                foreign_key_model: foreign_key,
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
                        col_type: rusqlite::types::Type::#col_type,
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
                    rusqlite::types::Value::#col_type(self.#name.clone().into())
                }
            });
    let query_set_method_sigs = model_fields
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
    let query_set_trait_methods = query_set_method_sigs.iter().cloned().map(|(fn_sig, _)| {
        quote! {
            #fn_sig;
        }
    });
    let query_set_trait_method_impls =
        query_set_method_sigs
            .iter()
            .cloned()
            .map(|(fn_sig, ModelFieldMeta { name, .. })| {
                let selector = format!("{name}=?");
                quote! {
                    #fn_sig {
                        let param = rusqlite::types::Value::from(value.into());
                        self.select(
                            #selector,
                            (param,),
                        )
                    }
                }
            });
    let query_set_trait_name = format_ident!("{model_name}QuerySetExt");
    let model_ext_trait_name = format_ident!("{model_name}Ext");
    let model_ext_method_sigs = model_fields
        .iter()
        .filter_map(|f| {
            let foreign_key_model = f.foreign_key_model.as_ref()?;
            let name = f.name.to_string();
            let fk_meth_name = &name[..name.len() - 3];
            let fk_meth_name = Ident::new(fk_meth_name, f.field.span());
            let set_fk_meth_name = Ident::new(&format!("set_{fk_meth_name}"), f.field.span());
            Some((
                [
                    quote! {
                        fn #fk_meth_name(&self) -> lit::Result<Option<#foreign_key_model>>
                    },
                    quote! {
                        fn #set_fk_meth_name(&mut self, instance: &#foreign_key_model)
                    },
                ],
                f,
            ))
        })
        .collect::<Vec<_>>();
    let model_ext_trait_methods = model_ext_method_sigs
        .iter()
        .flat_map(|(fn_sigs, _)| fn_sigs.clone().map(|sig| quote! {#sig;}));
    let model_ext_trait_method_impls =
        model_ext_method_sigs
            .iter()
            .map(|([fk_meth, set_fk_meth], f)| {
                let id_field_name = &f.name;
                let fk_model = f.foreign_key_model.as_ref().unwrap();
                quote! {
                    #fk_meth {
                        if self.#id_field_name == 0 {
                            return Ok(None);
                        }
                        Ok(
                            #fk_model::objects()
                            .select(
                                "id=?",
                                (self.#id_field_name,),
                            )?
                            .pop()
                        )
                    }

                    #set_fk_meth {
                        self.#id_field_name = instance.id;
                    }
                }
            });
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

            fn as_params(&self) -> std::vec::Vec<rusqlite::types::Value> {
                vec![
                    rusqlite::types::Value::Integer(self.id),
                    #(#values_quoted),*
                ]
            }

            fn from_row(row: impl IntoIterator<Item=rusqlite::types::Value>) -> rusqlite::types::FromSqlResult<Self> {
                let mut iter = row.into_iter();
                let mut next = || {
                    let Some(item) = iter.next() else {
                        return Err(rusqlite::types::FromSqlError::Other("not enough items".into()));
                    };
                    Ok(item)
                };
                Ok(Self {
                    id: rusqlite::types::FromSql::column_result((&next()?).into())?,
                    #(
                        #field_names: rusqlite::types::FromSql::column_result((&next()?).into())?
                    ),*
                })
            }
        }

        trait #model_ext_trait_name {
            #(#model_ext_trait_methods)*
        }

        impl #model_ext_trait_name for #model_name {
            #(#model_ext_trait_method_impls)*
        }

        trait #query_set_trait_name {
            #(#query_set_trait_methods)*
        }

        impl #query_set_trait_name for lit::query_set::QuerySet<#model_name> {
            #(#query_set_trait_method_impls)*
        }
    };

    tokens.into()
}

fn column_type_of(field: &Field) -> rusqlite::types::Type {
    match &field.ty {
        Type::Path(path) => {
            let first_segment = path.path.segments.first().unwrap();
            match first_segment.ident.to_string().as_str() {
                "String" => rusqlite::types::Type::Text,
                "i64" => rusqlite::types::Type::Integer,
                "f64" => rusqlite::types::Type::Real,
                "bool" => rusqlite::types::Type::Integer,
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
