extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{Expr, Ident, parse::{Parse, Result as ParseResult, ParseStream}, parse_macro_input, Token};

struct UsizeMatchInput {
    function_name: Ident,
    arg_name: Ident,
    match_body: Expr,
    else_body: Expr,
}

impl Parse for UsizeMatchInput {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let function_name = input.parse()?;
        input.parse::<Token![=]>()?;
        let arg_name = input.parse()?;
        input.parse::<Token![=>]>()?;
        let match_body = input.parse()?;
        input.parse::<Token![else]>()?;
        let else_body = input.parse()?;
        Ok(UsizeMatchInput {
            function_name,
            arg_name,
            match_body,
            else_body,
        })
    }
}

//#[inline]
//fn usize_size() -> u64 {
//    env!(
//        "CARGO_CFG_TARGET_POINTER_WIDTH",
//        "CARGO_CFG_TARGET_POINTER_WIDTH environment variable should be defined to know a target platform pointer size"
//    )
//        .parse()
//        .expect("CARGO_CFG_TARGET_POINTER_WIDTH environment variable should be an unsigned number")
//}

struct RangeIter {
    current: usize,
    next: Option<usize>,
    step: usize,
    max: usize,
}

impl RangeIter {
    #[inline]
    fn new(start: usize, step: usize, max: usize) -> Self {
        let next = start + step;
        let next = if next > max { None } else { Some(next) };
        Self {
            current: start,
            next,
            step,
            max,
        }
    }
}

impl Iterator for RangeIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.next {
            Some(x) => x,
            None => return None,
        };

        let current = self.current;
        self.current = next;
        let new_next = next + self.step;
        self.next = if new_next > self.max { None } else { Some(new_next) };

        Some((current, next))
    }
}

#[inline]
#[allow(dead_code)]
fn match_for(input: &UsizeMatchInput, pointer_width: usize) -> impl ToTokens {
    let UsizeMatchInput { arg_name, match_body, else_body, .. } = input;
    let mut variants = quote!(
        0 => {
            const #arg_name: usize = 0;
            #match_body
        }
    );

    let range = if pointer_width <= 8 {
        RangeIter::new(0, 1, 32)
            .chain(RangeIter::new(32, 8, 1 << (pointer_width - 1)))
    } else {
        RangeIter::new(0, 1, 1024)
            .chain(RangeIter::new(1024, 1024, 1048576.min(1 << (pointer_width - 1))))
    };
    for (min, max) in range {
        let min_exclusive = min + 1;
        variants = quote!(
            #variants
            #min_exclusive..=#max => {
                const #arg_name: usize = #max;
                #match_body
            }
        );
    }

    let pointer_width_formatted = format!("{}", pointer_width);

    quote!(
        #[cfg(target_pointer_width = #pointer_width_formatted)]
        return match input {
            #variants
            _ => #else_body,
        };
    )
}

#[proc_macro]
pub fn usize_match(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as UsizeMatchInput);
    // TODO CARGO_CFG_TARGET_POINTER_WIDTH
    let UsizeMatchInput { function_name, else_body, .. } = &input;
    let variant_8bit = match_for(&input, 8);
    let variant_16bit = match_for(&input, 16);
    let variant_32bit = match_for(&input, 32);
    let variant_64bit = match_for(&input, 64);
    let body = quote!(
        #variant_8bit
        #variant_16bit
        #variant_32bit
        #variant_64bit
        #[allow(dead_code)]
        #else_body
    );
    let body_str = body.to_string();
    let result = quote!(fn #function_name(input: usize) -> usize {
        println!("{}", #body_str);
        #body
    });
    TokenStream::from(result)
}
