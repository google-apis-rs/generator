use crate::Resource;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate(resource: &Resource) -> TokenStream {
    let parent_path = &resource.parent_path;
    let resource_ident = &resource.ident;
    let action_ident = resource.action_type_name();
    let description = format!(
        "Actions that can be performed on the {} resource",
        &resource.ident
    );
    quote! {
        #[doc= #description]
        pub fn #resource_ident(&self) -> #parent_path::#resource_ident::#action_ident {
            #parent_path::#resource_ident::#action_ident{
                reqwest: &self.reqwest,
                auth: self.auth_ref(),
            }
        }
    }
}
