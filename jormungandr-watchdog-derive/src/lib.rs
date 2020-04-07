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

// TODO: the settings type should be derived from the CoreServices impl
//       this to avoid issue with the competing names
fn gen_settings_type(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        quote! {
            #field_name: <#ty as ::jormungandr_watchdog::service::ManageService>::Settings
        }
    });

    quote! {
        #[derive(::std::default::Default, ::serde::Serialize, ::serde::Deserialize)]
        struct CoreServicesSettings {
            #( #cases ),*
        }
    }
}

fn gen_cli_args(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let cases = fields.iter().map(|field| {
        let ty = &field.ty;

        quote! {
            app = app.args(&
                <
                    <
                        #ty
                        as ::jormungandr_watchdog::service::ManageService
                    >::Settings
                    as ::jormungandr_watchdog::service::Settings
                >::add_cli_args()
            );
        }
    });

    quote! {
        fn add_cli_args<'a, 'b>(mut app: ::clap::App<'a, 'b>) -> ::clap::App<'a, 'b>
          where
            'a: 'b {
            #( #cases )*
            app
        }
    }
}

fn gen_new(fields: &Punctuated<Field, Comma>) -> TokenStream {
    let cases = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        quote! {
            #field_name: {
                let (rt, sm) = ::jormungandr_watchdog::service::ServiceManager::new(&mut settings.#field_name, args);
                runtimes.push(rt);
                sm
            }
        }
    });

    quote! {
        fn new<'a>(settings: &mut Self::Settings, args: &::clap::ArgMatches<'a>) -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
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
    let cli = gen_cli_args(fields);
    let settings_type = gen_settings_type(fields);
    let new = gen_new(fields);
    let start = gen_start(fields);
    let stop = gen_stop(fields);
    let status = gen_status(fields);
    let intercom = gen_intercom(fields);

    quote! {
        #settings_type

        #[async_trait::async_trait]
        #[allow(clippy::unit_arg)]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            type Settings = CoreServicesSettings;
            #cli
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
        #[allow(clippy::unit_arg)]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            type Settings = ();

            fn add_cli_args<'a, 'b>(app: ::clap::App<'a, 'b>) -> ::clap::App<'a, 'b>
            where
                'a: 'b {
                app
            }

            fn new<'a>(_: &mut Self::Settings, _: &::clap::ArgMatches<'a>) -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
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
        #[allow(clippy::unit_arg)]
        impl ::jormungandr_watchdog::CoreServices for #struct_name {
            type Settings = ();

            fn add_cli_args<'a, 'b>(_app: ::clap::App<'a, 'b>) -> ::clap::App<'a, 'b>
            where
                'a: 'b {
                unimplemented!()
            }

            fn new<'a>(_: &mut Self::Settings, args: &::clap::ArgMatches<'a>) -> (::std::vec::Vec<::tokio::runtime::Runtime>, Self) {
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


#[proc_macro_derive(IntercomMsg)]
pub fn derive_intercom_msg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Get syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    // Get the type name
    let type_name = &input.ident;
    // Build default impl
    let expanded = quote! {
        impl jormungandr_watchdog::service::IntercomMsg for #type_name {}
    };
    // Return TokenStream for default impl
    proc_macro::TokenStream::from(expanded)
}