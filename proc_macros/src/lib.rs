// A crate for procedure macros. It adds the ability to make advanced structure to further use in the OS.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Iternum)]
pub fn iternum(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    // Match on the data type to ensure the derive macro is used on an enum
    match ast.data {
        Data::Enum(data_enum) => {
            // This extracts the enum identifier
            let enum_ident = &ast.ident;

            // This extracts the variant names
            let variant_names = data_enum.variants.iter().filter_map(|variant| {
                let variant_ident = &variant.ident;
                match &variant.fields {
                    syn::Fields::Unit => {
                        // Variants without fields are returned simply.
                        Some(quote!(#enum_ident::#variant_ident))
                    }
                    _ => {
                        // TODO! Variants with fields must accept data from user.
                        // The values are ignored for now.
                        None
                    }
                }
            });

            // Get the count of enum variants to create an array of the correct size
            let count_variants = variant_names.clone().count();

            let gen = quote! {
                impl crate::kernel_components::structures::IternumTrait for #enum_ident {
                    const SIZE: usize = #count_variants;
                    
                    fn iter() -> [Self; #count_variants] {
                        [#(#variant_names),*]
                    }

                    fn get_index(variant: Self) -> usize {
                        let mut index = 0;
                        for var in #enum_ident::iter() {
                            if variant == var {
                                return index
                            } else {
                                index += 1;
                            }
                        }
                        usize::MAX
                    }

                    fn get_variant(index: usize) -> Self {
                        #enum_ident::iter()[index]
                    }

                    fn get_size() -> usize { #enum_ident::SIZE }
                }
            };

            gen.into()
        }
        _ => panic!("Iternum can only be derived for enums."),
    }
}






