use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(HelloMacro)]
pub fn packet_into_bytes(input: TokenStream) -> TokenStream {
    let ast = (&&syn::parse(input)).unwrap();

    packet_into_bytes_impl(ast)
}

fn packet_into_bytes_impl(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let generated = quote!();
    generated.into()
}
