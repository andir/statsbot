use proc_macro::TokenStream;

use quote::quote;

use syn::{Data, DeriveInput, Fields, Lit, Meta, MetaNameValue, parse_macro_input};

#[proc_macro_derive(SimplePrometheus, attributes(prefix))]
pub fn simple_prometheus_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match &ast.data {
        Data::Struct(s) => s.fields.clone(),
        _ => unimplemented!(),
    };

    let prefix = get_prefix(&ast.attrs)
        .map(|prefix| {
            if !prefix.starts_with("_") {
                format!("{}_", prefix)
            } else {
                prefix
            }
        })
        .unwrap_or("".to_string());

    let name = &ast.ident;

    let field_names = match fields {
        Fields::Named(field_named) => field_named.named.into_iter().map(|f| f.ident.unwrap()),
        _ => unimplemented!(),
    };

    let write_statements = field_names.map(|field_name| {
        let prefix = format!("{}{}", prefix, field_name);
        let format_label = format!("{}{} {}", prefix, "{{server=\"{}\"}}", "{}");
        let format = format!("{} {}", prefix, "{}");

        #[rustfmt::skip]
        quote! {
            if let Some(ref server) = server {
		writeln!(out, #format_label, server, self.#field_name)?;
            } else {
		writeln!(out, #format, self.#field_name)?;
            }
        }
    });

    #[rustfmt::skip]
    let code = quote! {
	impl simple_prometheus::SimplePrometheus for #name {
	    fn to_prometheus_metrics(&self, server: Option<String>) -> Result<String, core::fmt::Error> {
		use std::fmt::Write;
		let mut out = String::new();
		#( #write_statements )*

		Ok(out)
	    }
	}
    };
    //println!("{}", code.to_string());
    code.into()
}

fn get_prefix(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("simple_prometheus"))
        .filter_map(|attr| {
            if let Meta::NameValue(MetaNameValue {
                path,
                value: syn::Expr::Lit(expr_lit),
                ..
            }) = attr.meta.clone()
            {
                Some((path, expr_lit))
            } else {
                None
            }
        })
        .filter_map(|(path, attr)| {
            if path.is_ident("prefix") {
                if let Lit::Str(lit_str) = &attr.lit {
                    Some(lit_str.value())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .next()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_simple() {}
}
