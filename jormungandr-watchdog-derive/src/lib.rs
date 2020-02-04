extern crate proc_macro;
use proc_macro2::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error, set_dummy};
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, *};

#[proc_macro_derive(CoreServices)]
#[proc_macro_error]
pub fn derive_core_services(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let gen = impl_core_services(&input);
    gen.into()
}

fn gen_new(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        quote! {
            #field_name: {
                let (rt, sm) = ::jormungandr_watchdog::service::ServiceManager::new();
                runtimes.push(rt);
                sm
            }
        }
    });

    quote! {
        fn new() -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
            let mut runtimes = ::std::vec::Vec::new();

            let entity = Self {
                #( #cases ),*
            };

            (runtimes, entity)
        }
    }
}

fn gen_start(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let possible_values = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry
        }
    });

    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry => {
                match self.#field_name.runtime(watchdog_query) {
                    Ok(rt) => Ok(rt.start()),
                    Err(source) => Err(::jormungandr_watchdog::WatchdogError::CannotStartService {
                        service_identifier,
                        source,
                    })
                }
            }
        }
    });

    quote! {
        fn start(
            &mut self,
            service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
            watchdog_query: ::jormungandr_watchdog::WatchdogQuery,
        ) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
            match service_identifier {
                #( #cases ),*
                _ => Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[#( #possible_values ),*],
                })
            }
        }
    }
}

fn gen_stop(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let possible_values = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry
        }
    });

    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry => { Ok(self.#field_name.shutdown()) }
        }
    });

    quote! {
        fn stop(
            &mut self,
            service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
        ) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
            match service_identifier {
                #( #cases ),*
                _ => Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[#( #possible_values ),*],
                })
            }
        }
    }
}

fn gen_intercom(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let possible_values = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry
        }
    });

    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry => { Ok(Box::new(self.#field_name.intercom())) }
        }
    });

    quote! {
        fn intercoms(
            &mut self,
            service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
        ) -> Result<Box<dyn ::std::any::Any + Send>, ::jormungandr_watchdog::WatchdogError> {
            match service_identifier {
                #( #cases ),*
                _ => Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[#( #possible_values ),*],
                })
            }
        }
    }
}

fn gen_status(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let possible_values = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry
        }
    });

    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        let entry = field_name.to_string();

        quote! {
            #entry => { Ok(self.#field_name.status().await) }
        }
    });

    quote! {
        async fn status(
            &mut self,
            service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
        ) -> Result<::jormungandr_watchdog::service::StatusReport, ::jormungandr_watchdog::WatchdogError> {
            match service_identifier {
                #( #cases ),*
                _ => Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[#( #possible_values ),*],
                })
            }
        }
    }
}

fn impl_core_services_for_struct(
    struct_name: &Ident,
    fields: &Punctuated<Field, Comma>,
) -> TokenStream {
    let new = gen_new(fields);
    let start = gen_start(fields);
    let stop = gen_stop(fields);
    let status = gen_status(fields);
    let intercom = gen_intercom(fields);

    quote! {
        #[async_trait::async_trait]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            #new
            #start
            #status
            #stop
            #intercom
        }
    }
}

// create the impl of CoreServices for Unit Structure
fn impl_core_services_for_struct_unit(struct_name: &Ident) -> TokenStream {
    quote! {
        #[async_trait::async_trait]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            fn new() -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
                (::std::vec::Vec::new(), Self)
            }

            fn start(
                &mut self,
                service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
                _watchdog_query: ::jormungandr_watchdog::WatchdogQuery,
            ) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
                Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[],
                })
            }
            async fn status(
                &mut self,
                service_identifier: ::jormungandr_watchdog::ServiceIdentifier
            ) -> Result<::jormungandr_watchdog::service::StatusReport, ::jormungandr_watchdog::WatchdogError> {
                Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[],
                })
            }
            fn stop(&mut self, service_identifier: ::jormungandr_watchdog::ServiceIdentifier) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
                Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[],
                })
            }
            fn intercoms(
                &mut self,
                service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
            ) -> Result<Box<dyn Any + Send>, ::jormungandr_watchdog::WatchdogError> {
                Err(::jormungandr_watchdog::WatchdogError::UnknownService {
                    service_identifier,
                    possible_values: &[],
                })
            }
        }
    }
}

fn impl_core_services(input: &DeriveInput) -> TokenStream {
    use syn::Data::*;

    let struct_name = &input.ident;

    set_dummy(quote! {
        #[async_trait::async_trait]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            fn new() -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
                unimplemented!()
            }
            fn start(
                &mut self,
                _service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
                _watchdog_query: ::jormungandr_watchdog::WatchdogQuery,
            ) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
                unimplemented!()
            }
            fn stop(
                &mut self,
                _service_identifier: ::jormungandr_watchdog::ServiceIdentifier
            ) -> Result<(), ::jormungandr_watchdog::WatchdogError> {
                unimplemented!()
            }
            async fn status(
                &mut self,
                _service_identifier: ::jormungandr_watchdog::ServiceIdentifier
            ) -> Result<::jormungandr_watchdog::service::StatusReport, ::jormungandr_watchdog::WatchdogError> {
                unimplemented!()
            }
            fn intercoms(
                &mut self,
                _service_identifier: ::jormungandr_watchdog::ServiceIdentifier,
            ) -> Result<Box<dyn ::std::any::Any + Send>, ::jormungandr_watchdog::WatchdogError> {
                unimplemented!()
            }
        }
    });

    match input.data {
        Struct(DataStruct {
            fields: syn::Fields::Named(ref fields),
            ..
        }) => impl_core_services_for_struct(&struct_name, &fields.named),
        Struct(DataStruct {
            fields: syn::Fields::Unit,
            ..
        }) => impl_core_services_for_struct_unit(&struct_name),
        _ => abort_call_site!("CoreServices only supports non-tuple struct"),
    }
}
