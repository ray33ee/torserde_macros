
use proc_macro2::{TokenStream, Ident, Span};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Index};

#[proc_macro_derive(Torserde)]
pub fn torserde_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let serialiser = tor_serialise(&input.data);
    let deserialiser = tor_deserialise(&input.data);
    let get_length = tor_get_length(&input.data);

    let expanded = quote! {
        impl torserde::TorSerde for #name {
            fn bin_serialise_into<W: std::io::Write>(&self, mut stream: W) {
                #serialiser
            }

            fn bin_deserialise_from<R: std::io::Read>(mut stream: R, payload_length: Option<u32>) -> Self {
                #deserialiser
            }

            fn serialised_length(&self) -> u32 {
                #get_length
            }
        }
    };

    proc_macro::TokenStream::from(expanded)

}

fn tor_serialise(data: &Data) -> TokenStream {

    //todo: If we use repr to specify the length of the discriminant, update u8's to reflect this
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! {f.span()=>
                        torserde::TorSerde::bin_serialise_into(&self.#name, std::borrow::BorrowMut::borrow_mut(& mut stream));
                    }
                });
                quote! {
                    #(#recurse)*
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! {f.span()=>
                        torserde::TorSerde::bin_serialise_into(&self.#index, std::borrow::BorrowMut::borrow_mut(& mut stream));
                    }
                });
                quote! {
                    #(#recurse)*
                }
            }
            Fields::Unit => {
                panic!("Torserde cannot serialise/deserialise unit structures")
            }
        }
        Data::Enum(ref data) => {
            let arms = data.variants.iter().map(|v| {
                let enum_ident = &v.ident;

                let discriminant = &v.discriminant.as_ref().unwrap().1;

                let (identifiers, match_arm) = match &v.fields {
                    Fields::Named(ref named) => {
                        let idents = named.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! {f.span()=>
                                #name
                            }
                        });
                        let idents2 = idents.clone();

                        (
                            quote! {
                                { #(#idents,)* }
                            },
                            quote! {
                                (#discriminant as u8).bin_serialise_into(std::borrow::BorrowMut::borrow_mut(& mut stream));
                                #(#idents2.bin_serialise_into(std::borrow::BorrowMut::borrow_mut(& mut stream));)*
                            }
                        )
                    }
                    Fields::Unnamed(ref unnamed) => {
                        let idents = unnamed.unnamed.iter().enumerate().map(|(i, _)| {
                            let number = Ident::new(format!("v_{}", i).as_str(), Span::call_site());
                            quote! {//f.span()=>
                                #number
                            }
                        });

                        let idents2 = idents.clone();

                        (
                            quote! {
                                ( #(#idents,)* )
                            },
                            quote! {
                                (#discriminant as u8).bin_serialise_into(std::borrow::BorrowMut::borrow_mut(& mut stream));
                                #(#idents2.bin_serialise_into(std::borrow::BorrowMut::borrow_mut(& mut stream));)*
                            }
                        )
                    }
                    Fields::Unit => {
                        (
                            quote! {

                            },
                            quote! {
                                (#discriminant as u8).bin_serialise_into(std::borrow::BorrowMut::borrow_mut(& mut stream));
                            }
                        )
                    }
                };

                quote_spanned! {v.span()=>
                    Self::#enum_ident#identifiers => { #match_arm }
                }
            });

            quote! {
                match &self {
                    #(#arms)*
                }
            }

        }
        Data::Union(_) => {
            panic!("Torserde does not support serialisation of Rust Unions")
        }
    }
}



fn tor_deserialise(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! {f.span()=>
                        #name: torserde::TorSerde::bin_deserialise_from(std::borrow::BorrowMut::borrow_mut(& mut stream), payload_length),
                    }
                });
                quote! {
                    Self {
                        #(#recurse)*
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().map(|f| {
                    quote_spanned! {f.span()=>
                        torserde::TorSerde::bin_deserialise_from(std::borrow::BorrowMut::borrow_mut(& mut stream), payload_length),
                    }
                });
                quote! {
                    Self(#(#recurse)*)
                }
            }
            Fields::Unit => {
                panic!("Torserde cannot serialise/deserialise unit structures")
            }
        }
        Data::Enum(ref data) => {
            let arms = data.variants.iter().map(|v| {
                let enum_ident = &v.ident;

                let discriminant = &v.discriminant.as_ref().unwrap().1;

                let match_arm = match &v.fields {
                    Fields::Named(ref named) => {
                        let idents = named.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! {f.span()=>
                                #name
                            }
                        });

                        quote! {
                            { #(#idents: torserde::TorSerde::bin_deserialise_from(std::borrow::BorrowMut::borrow_mut(& mut stream), payload_length),)* }
                        }

                    }
                    Fields::Unnamed(ref unnamed) => {

                        let idents = unnamed.unnamed.iter().map(|_| quote! {});

                        quote! {
                            ( #(#idents torserde::TorSerde::bin_deserialise_from(std::borrow::BorrowMut::borrow_mut(& mut stream), payload_length), )* )
                        }

                    }
                    Fields::Unit => {
                        quote! {

                        }
                    }
                };

                quote_spanned! {v.span()=>
                    #discriminant => Self::#enum_ident#match_arm,
                }
            });

            quote! {
                let discriminant = u8::bin_deserialise_from(std::borrow::BorrowMut::borrow_mut(& mut stream), payload_length);

                match discriminant {
                    #(#arms)*
                    _ => { panic!("Invalid discriminant") }
                }
            }
        }
        Data::Union(_) => {
            panic!("Torserde does not support serialisation of Rust Unions")
        }
    }
}



fn tor_get_length(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! {f.span()=>
                        torserde::TorSerde::serialised_length(&self.#name)
                    }
                });
                quote! {
                    0 #(+ #recurse)*
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! {f.span()=>
                        torserde::TorSerde::serialised_length(&self.#index)
                    }
                });
                quote! {
                    0 #(+ #recurse)*
                }
            }
            Fields::Unit => {
                panic!("Torserde cannot serialise/deserialise unit structures")
            }
        }
        Data::Enum(ref data) => {
            let arms = data.variants.iter().map(|v| {
                let enum_ident = &v.ident;

                let (identifiers, match_arm) = match &v.fields {
                    Fields::Named(ref named) => {
                        let idents = named.named.iter().map(|f| {
                            let name = &f.ident;
                            quote_spanned! {f.span()=>
                                #name
                            }
                        });
                        let idents2 = idents.clone();

                        (
                            quote! {
                                { #(#idents,)* }
                            },
                            quote! {
                                0 #(+ #idents2.serialised_length())*
                            }
                        )
                    }
                    Fields::Unnamed(ref unnamed) => {
                        let idents = unnamed.unnamed.iter().enumerate().map(|(i, _)| {
                            let number = Ident::new(format!("v_{}", i).as_str(), Span::call_site());
                            quote! {//f.span()=>
                                #number
                            }
                        });

                        let idents2 = idents.clone();

                        (
                            quote! {
                                ( #(#idents,)* )
                            },
                            quote! {
                                0 #(+ #idents2.serialised_length())*
                            }
                        )
                    }
                    Fields::Unit => {
                        (
                            quote! {

                            },
                            quote! {
                                0
                            }
                        )
                    }
                };

                quote_spanned! {v.span()=>
                    Self::#enum_ident#identifiers => { #match_arm }
                }
            });

            //todo: If we use repr to specify the length of the discriminant, update 1 to reflect this
            quote! {
                1 + match &self {
                    #(#arms)*
                }
            }
        }
        Data::Union(_) => {
            panic!("Torserde does not support serialisation of Rust Unions")
        }
    }
}
