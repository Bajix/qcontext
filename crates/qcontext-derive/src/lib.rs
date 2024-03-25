use darling::FromDeriveInput;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(context))]
struct ContextDeriveInput {
  ident: Ident,
  state: Type,
}

/// Implements [Context](https://docs.rs/qcontext/latest/qcontext/trait.Context.html)
///
/// ## Attributes
///
/// * `#[context(state = "TCell<Context, usize>")]` sets [`Context::State`](https://docs.rs/qcontext/latest/qcontext/trait.Context.htmll#associatedtype.State)
#[proc_macro_derive(Context, attributes(context))]
pub fn derive_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let ContextDeriveInput { ident, state } = match ContextDeriveInput::from_derive_input(&input) {
    Ok(input) => input,
    Err(err) => return err.write_errors().into(),
  };

  let expanded = quote! {
    impl qcontext::Context for #ident {
      type State = #state;

      fn context() -> &'static qcontext::OnceCell<Self::State> {
        static CONTEXT: qcontext::OnceCell<#state> = qcontext::OnceCell::new();

        &CONTEXT
      }
    }
  };

  expanded.into()
}
