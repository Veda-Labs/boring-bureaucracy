use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Path, Type, TypePath, parse_macro_input};

#[proc_macro_derive(BuildingBlockCache, attributes(can_derive))]
pub fn derive_building_block_cache(input: TokenStream) -> TokenStream {
    // Parse the input
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Get the fields of the struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new(
                    input.span(),
                    "BuildingBlockCache only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new(input.span(), "BuildingBlockCache only supports structs")
                .to_compile_error()
                .into();
        }
    };

    // Build the block_name using the struct name (for error reporting)
    let block_name = format!(
        "{}_block",
        struct_name.to_string().to_lowercase().replace("block", "")
    );

    // Process fields for each method
    let mut resolve_values = Vec::new();
    let mut report_missing_values = Vec::new();
    let mut derive_value_match_arms = Vec::new();
    let mut required_derive_methods = Vec::new();

    for field in fields {
        if let Some(field_name) = &field.ident {
            let field_name_str = field_name.to_string();

            // Check if field has the can_derive attribute
            let can_derive = field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("can_derive"));

            // Generate resolve_values code based on field type
            if let Type::Path(TypePath { path, .. }) = &field.ty {
                if is_option_type(path) {
                    let inner_type = get_option_inner_type(path);

                    // Generate code based on inner type
                    if inner_type == "Address" {
                        resolve_values.push(quote! {
                            if let Some(value) = self.#field_name {
                                cache.set(#field_name_str, CacheValue::Address(value), #block_name).await?;
                            }
                        });
                    } else if inner_type == "u32" {
                        resolve_values.push(quote! {
                            if let Some(value) = self.#field_name {
                                cache.set(#field_name_str, CacheValue::U32(value), #block_name).await?;
                            }
                        });
                    } else if inner_type == "u8" {
                        resolve_values.push(quote! {
                            if let Some(value) = self.#field_name {
                                cache.set(#field_name_str, CacheValue::U8(value), #block_name).await?;
                            }
                        });
                    } else if inner_type == "String" {
                        resolve_values.push(quote! {
                            if let Some(ref value) = self.#field_name {
                                cache.set(#field_name_str, CacheValue::String(value.clone()), #block_name).await?;
                            }
                        });
                    } else if inner_type == "bool" {
                        resolve_values.push(quote! {
                            if let Some(value) = self.#field_name {
                                cache.set(#field_name_str, CacheValue::Bool(value), #block_name).await?;
                            }
                        });
                    } else if inner_type == "AddressOrContractName" {
                        resolve_values.push(quote! {
                            if let Some(ref value) = self.#field_name {
                                match value {
                                    AddressOrContractName::Address(addr) => {
                                        cache.set(#field_name_str, CacheValue::Address(*addr), #block_name).await?;
                                    },
                                    AddressOrContractName::ContractName(name) => {
                                        if let Some(deployer) = self.deployer {
                                            let addr = derive_contract_address(name, deployer);
                                            cache.set(#field_name_str, CacheValue::Address(addr), #block_name).await?;
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }

            // Generate report_missing_values code
            report_missing_values.push(quote! {
                if self.#field_name.is_none() && cache.get(#field_name_str).await.is_none() {
                    requires.push((#field_name_str.to_string(), #can_derive));
                }
            });

            // Generate derive_value match arms for derivable fields
            if can_derive {
                let derive_method_name = format_ident!("derive_{}", field_name);
                derive_value_match_arms.push(quote! {
                    #field_name_str => self.#derive_method_name(cache, vrm).await,
                });

                // Track required derive methods
                required_derive_methods.push((field_name_str.clone(), derive_method_name));
            }
        }
    }

    // Generate the resolve_values method
    let resolve_values_impl = quote! {
        async fn resolve_values(&self, cache: &SharedCache) -> Result<()> {
            #(#resolve_values)*
            Ok(())
        }
    };

    // Generate the report_missing_values method
    let report_missing_values_impl = quote! {
        async fn report_missing_values(&self, cache: &SharedCache) -> Result<Vec<(String, bool)>> {
            let mut requires = Vec::new();
            #(#report_missing_values)*
            Ok(requires)
        }
    };

    // Generate the derive_value method
    let derive_value_impl = quote! {
        async fn derive_value(
            &self,
            key: &str,
            cache: &SharedCache,
            vrm: &ViewRequestManager,
        ) -> Result<bool> {
            match key {
                #(#derive_value_match_arms)*
                _ => {
                    return Err(eyre::eyre!(
                        "{}: Requested derivation of {}, which is not supported",
                        #block_name, key
                    ));
                }
            }
        }
    };

    // Generate the assemble method that calls _assemble
    let assemble_impl = quote! {
        async fn assemble(&self, cache: &SharedCache, vrm: &ViewRequestManager) -> Result<Vec<Box<dyn Action>>> {
            self._assemble(cache, vrm).await
        }
    };

    // Generate implementation
    let expanded = quote! {
        #[async_trait::async_trait]
        impl BuildingBlock for #struct_name {
            #assemble_impl
            #resolve_values_impl
            #report_missing_values_impl
            #derive_value_impl
        }
    };

    // Generate compile-time checks for required methods
    let mut method_checks = proc_macro2::TokenStream::new();

    // Add check for _assemble method
    method_checks.extend(quote! {
        // Force compilation error if _assemble method is missing
        const _: fn() = || {
            let _ = #struct_name::_assemble;
        };
    });

    // Add checks for all derive methods
    for (_, method_name) in required_derive_methods {
        method_checks.extend(quote! {
            // Force compilation error if derive method is missing
            const _: fn() = || {
                let _ = #struct_name::#method_name;
            };
        });
    }

    let final_output = quote! {
        #expanded

        const _: () = {
            #method_checks
        };
    };

    final_output.into()
}

// Helper function to check if a type is Option<T>
fn is_option_type(path: &Path) -> bool {
    if path.segments.len() == 1 {
        let segment = &path.segments[0];
        segment.ident == "Option"
    } else {
        false
    }
}

// Helper function to get the inner type of an Option<T>
fn get_option_inner_type(path: &Path) -> String {
    if let Some(segment) = path.segments.first() {
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(Type::Path(type_path))) = args.args.first() {
                if let Some(inner_segment) = type_path.path.segments.last() {
                    return inner_segment.ident.to_string();
                }
            }
        }
    }
    String::new()
}
