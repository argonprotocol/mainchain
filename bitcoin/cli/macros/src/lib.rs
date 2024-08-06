use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, ExprLit, Lit};

#[proc_macro_derive(ReadDocs)]
pub fn read_docs_derive(input: TokenStream) -> TokenStream {
	// Parse the input tokens into a syntax tree
	let input = parse_macro_input!(input as DeriveInput);

	// Get the name of the struct
	let name = &input.ident;

	// Generate the implementation
	let expanded = match input.data {
		syn::Data::Struct(data_struct) => {
			let mut fields_with_docs = Vec::new();

			// Iterate through each field in the struct
			for field in data_struct.fields {
				if let Some(ident) = field.ident {
					// Extract the documentation attributes
					let docs: Vec<String> = get_docs(&field.attrs)
						.iter()
						.map(|expr| {
							if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = expr {
								lit_str.value()
							} else {
								"".to_string()
							}
						})
						.collect();

					// Combine docs into a single string
					let docs_combined = docs.join("\n");
					let field_name = ident.to_string();

					fields_with_docs.push(quote! {
						#field_name => Some(#docs_combined),
					});
				}
			}

			quote! {
				impl #name {
					pub fn get_docs(field: &str) -> Option<&'static str> {
						match field {
							#( #fields_with_docs )*
							_ => None,
						}
					}
				}
			}
		},
		_ => quote! {},
	};

	// Convert the expanded code back to a token stream and return it
	TokenStream::from(expanded)
}

fn get_docs(attrs: &[syn::Attribute]) -> Vec<syn::Expr> {
	attrs
		.iter()
		.filter_map(|attr| {
			if let syn::Meta::NameValue(meta) = &attr.meta {
				meta.path
					.get_ident()
					.filter(|ident| *ident == "doc")
					.map(|_| meta.value.clone())
			} else {
				None
			}
		})
		.collect()
}
