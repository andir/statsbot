#![allow(non_upper_case_globals)] // rustc bug: https://github.com/rust-lang/rust/issues/110573
#![allow(clippy::wildcard_imports)]

use quote::quote;
use reflect::*;

library! {
    use zero {
        trait Zero {
            fn zero();
        }
    }
}

fn derive(ex: Execution) {
    ex.make_trait_impl(RUNTIME::zero::Zero, ex.target_type(), |block| {
        block.make_function(RUNTIME::zero::Zero::zero, |make_function| {
            make_function.unit()
        });
    });
}

#[test]
fn test_zero_args() {
    let input = quote! {
        struct Zero {
            pub zero: ()
        }
    };

    let expected = quote! {
        impl::zero::Zero for Zero {
            fn zero() {
                let __v0 = ();
                __v0
            }
        }
    };

    let output = reflect::derive(input, derive);
    assert_eq!(output.to_string(), expected.to_string());
}
