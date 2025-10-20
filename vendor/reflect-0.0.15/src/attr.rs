use quote::ToTokens;
use ref_cast::RefCast;
use std::fmt::{self, Debug};
use syn::{AttrStyle, Attribute, Meta};

#[allow(clippy::ptr_arg)]
pub fn debug(attrs: &Vec<Attribute>) -> &impl Debug {
    Wrapper::ref_cast(attrs)
}

#[derive(RefCast)]
#[repr(transparent)]
struct Wrapper<T>(T);

impl Debug for Wrapper<Vec<Attribute>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(Wrapper::ref_cast))
            .finish()
    }
}

impl Debug for Wrapper<Attribute> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("#")?;
        match self.0.style {
            AttrStyle::Outer => {}
            AttrStyle::Inner(_) => f.write_str("!")?,
        }
        f.write_str("[")?;
        for (i, segment) in self.0.path().segments.iter().enumerate() {
            if i > 0 || self.0.path().leading_colon.is_some() {
                f.write_str("::")?;
            }
            write!(f, "{}", segment.ident)?;
        }
        match &self.0.meta {
            Meta::Path(_) => {}
            Meta::List(meta) => write!(f, "({})", meta.tokens)?,
            Meta::NameValue(meta) => write!(f, " = {}", meta.value.to_token_stream())?,
        }
        f.write_str("]")?;
        Ok(())
    }
}

#[test]
fn test_debug() {
    use syn::parse_quote;

    let attrs = vec![
        parse_quote!(#[derive(Debug)]),
        parse_quote!(#[doc = "..."]),
        parse_quote!(#[rustfmt::skip]),
    ];

    let actual = format!("{:#?}", debug(&attrs));
    let expected = "[\
                    \n    #[derive(Debug)],\
                    \n    #[doc = \"...\"],\
                    \n    #[rustfmt::skip],\
                    \n]";
    assert_eq!(actual, expected);
}
