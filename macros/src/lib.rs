use std::fmt::format;

use proc_macro::{TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, Ident, LitStr, Expr, parse::{ParseStream, Parse}, punctuated::Punctuated, parenthesized, token::Token};

/// This macro is used to set specific registers to specific values via a previously crated [RegisterCapture](crate::RegisterCapture).
/// 
/// # Syntax
/// 
/// ```ignore
/// __capture_set_registers!((<Register>, <Register>, <Register>, ...), <RegisterCapture>, <StackFrame>);
/// ```
/// 
/// **This is a proc macro and must be used with the `#[macro_use]` attribute.**
#[proc_macro]
pub fn __capture_set_registers(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ApplyRegistersFromCaptureInput);

    let capture = input.capture.clone();

    
    let expanded =quote::quote! {
        let __macro_capture = #capture;
        ::core::arch::asm!(
            "nop",
            #input
            options(nostack, nomem, preserves_flags)
        );
    }; 

    TokenStream::from(expanded)
}

struct ApplyRegistersFromCaptureInput {
    registers: Vec<Register>,
    capture: Expr,
}


impl Parse for ApplyRegistersFromCaptureInput {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let registers;
        let capture;

        let content;
        parenthesized!(content in input);


        registers = Punctuated::<Register, syn::Token![,]>::parse_terminated(&content)?;

        let _ = input.parse::<syn::Token![,]>()?;

        capture = input.parse()?;

        Ok(ApplyRegistersFromCaptureInput {
            registers: registers.into_iter().collect(),
            capture,
        })
    }
}

impl ToTokens for ApplyRegistersFromCaptureInput {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let registers = &self.registers;
        let capture = &self.capture;

        for register in registers {
            let name = &register.name;
            let name_str = name.to_string();
            tokens.extend(quote! {
                in(#name_str) #capture.#name,
            });
        }
    }
}

struct Register {
    name: Ident
}

impl Parse for Register {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let ident = input.parse()?;

        Ok(Register {
            name: ident,
        })
    }
}