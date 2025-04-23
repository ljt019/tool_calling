extern crate proc_macro;
use linkme::distributed_slice;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Meta};

/// Attribute macro that marks a function as a tool
///
/// # Example
/// ```
/// #[tool(description = "Get user info from database")]
/// pub fn get_user_info(user_id: u32) -> String {
///     // implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the description
    let mut description = String::new();

    if let Meta::NameValue(nv) = parse_macro_input!(args as Meta) {
        if nv.path.is_ident("description") {
            if let syn::Expr::Lit(expr_lit) = &nv.value {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    description = lit_str.value();
                }
            }
        }
    }

    // Parse the function itself
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.to_string();
    let fn_ident = &input_fn.sig.ident;
    let sig = &input_fn.sig;
    let block = &input_fn.block;

    // Extract parameter list as Vec<String>
    let params = sig
        .inputs
        .iter()
        .map(|arg| quote!(#arg).to_string())
        .collect::<Vec<_>>();

    // Get input parameters for parsing arguments
    let mut param_types = Vec::new();
    let mut param_names = Vec::new();

    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = &pat_ident.ident;
                param_names.push(param_name);
                param_types.push(&pat_type.ty);
            }
        }
    }

    // Count the parameters directly
    let param_count = param_names.len();

    // Generate a constructor function for the Tool rather than using static initialization
    let metadata_fn = syn::Ident::new(&format!("__register_tool_{}", fn_name), fn_ident.span());
    // Generate the appropriate closure based on parameter count
    let func_body = if param_count == 0 {
        quote! {
            |args: &[String]| {
                if !args.is_empty() {
                    return Err(format!("Expected 0 arguments, got {}", args.len()));
                }
                let result = #fn_ident();
                Ok(result)
            }
        }
    } else if param_count == 1 {
        quote! {
            |args: &[String]| {
                if args.len() != 1 {
                    return Err(format!("Expected 1 argument, got {}", args.len()));
                }
                if let Ok(arg0) = args[0].parse() {
                    let result = #fn_ident(arg0);
                    Ok(result)
                } else {
                    Err(format!("Failed to parse argument '{}'", args[0]))
                }
            }
        }
    } else {
        quote! {
            |_: &[String]| {
                Err("Functions with multiple arguments are not fully supported yet".to_string())
            }
        }
    };

    let expanded = quote! {
        #input_fn

        #[doc(hidden)]
        #[distributed_slice(tool_calling::TOOL_FACTORIES)]
        fn #metadata_fn() -> tool_calling::Tool {
            tool_calling::Tool {
                name: #fn_name.to_string(),
                description: #description.to_string(),
                parameters: vec![#( #params.to_string() ),*],
                function: #func_body,
            }
        }
    };

    expanded.into()
}
