#![feature(proc_macro)]
#![allow(unused_extern_crates)]
#![allow(non_snake_case)]
#![recursion_limit="128"]
extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::*;

#[proc_macro_attribute]
pub fn service(_attribute: TokenStream, function: TokenStream) -> TokenStream {
	let Item { node, .. } = syn::parse(function)
		.expect("failed to parse tokens");
	let ItemFn {
		ident,
		block,
		..
	} = match node {
		ItemKind::Fn(item) => item,
		_ => panic!("#[service]: Whoops!, try again. This time, with a function."),
	};
	
	let block = block.stmts.iter();
	
	let output = quote! {
		#[derive(Clone)]
		struct #ident;
		
		impl ArcService for #ident {
			fn call(&self, req: Request) -> Box<Future<Item = Response, Error =Error>> {
				Box::new(
					async_block!{
						#(
							#block
						)*
					}
				)
			}

			fn boxedClone(&self) -> Box<ArcService> {
				Box::new(self.clone())
			}
		}
	};

	output.to_string().parse().unwrap()
}