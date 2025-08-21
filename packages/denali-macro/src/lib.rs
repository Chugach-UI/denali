use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn gen_code(_input: TokenStream) -> TokenStream {
    let protocols_dir = std::env::var("OUT_DIR").unwrap();
    println!("Protocols directory is {}", protocols_dir);
    quote! {}
}
