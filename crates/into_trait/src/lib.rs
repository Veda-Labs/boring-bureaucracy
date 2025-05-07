extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DeriveInput};

#[proc_macro_derive(IntoTraitObject, attributes(trait_name))]
pub fn derive_into_trait_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let enum_name = &input.ident;

    // Find the trait_name attribute
    let mut trait_name: Option<proc_macro2::TokenStream> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("trait_name") {
            attr.parse_nested_meta(|meta| {
                if let Some(path) = meta.path.get_ident() {
                    trait_name = Some(quote! { #path });
                }
                Ok(())
            })
            .unwrap();
        }
    }
    let trait_name = trait_name.expect("You must specify #[trait_name(TraitName)] on the enum when using #[derive(IntoTraitObject)]");

    let Data::Enum(DataEnum { variants, .. }) = &input.data else {
        panic!("IntoTraitObject can only be derived for enums");
    };

    let arms = variants.iter().map(|v| {
        let vname = &v.ident;
        quote! {
            #enum_name::#vname(data) => Box::new(data) as Box<dyn #trait_name>
        }
    });

    let expanded = quote! {
        impl #enum_name {
            pub fn into_trait_object(self) -> Box<dyn #trait_name> {
                match self {
                    #(#arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
