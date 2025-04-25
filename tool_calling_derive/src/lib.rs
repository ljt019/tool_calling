extern crate proc_macro;
use linkme::distributed_slice;
use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Span};
use quote::quote;
use serde_json::json;
use syn::{parse_macro_input, FnArg, Ident, ItemFn, Meta, Pat, PatType, Type};

/// Attribute macro that marks a function as a tool
///
/// # Example
/// ```
/// Get user info from database
/// #[tool]
/// pub fn get_user_info(user_id: u32) -> String {
///     // implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(_args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function itself
    let input_fn = parse_macro_input!(item as ItemFn);
    // Extract documentation comments as description
    let mut description = String::new();
    for attr in &input_fn.attrs {
        if let Meta::NameValue(nv) = &attr.meta {
            // Only consider `#[doc = "..."]` attributes
            if nv.path.is_ident("doc") {
                // The value of MetaNameValue is an expression
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let doc_string = lit_str.value();
                        let line = doc_string.trim();
                        if !description.is_empty() {
                            description.push('\n');
                        }
                        description.push_str(line);
                    }
                }
            }
        }
    }

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
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                param_names.push(param_name);
                param_types.push(&pat_type.ty);
            }
        }
    }

    // Build JSON Schema for parameters
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                required.push(json!(param_name.clone())); // Add name to required list

                // Convert the type to a string and clean up any whitespace/references
                let type_str = quote!(#pat_type.ty).to_string().replace(" ", "");
                let json_type = match type_str.as_str() {
                    // Match on integer types
                    s if s.starts_with("u")
                        || s.starts_with("i")
                        || s == "usize"
                        || s == "isize" =>
                    {
                        "integer"
                    }
                    // Match on floating point types
                    "f32" | "f64" => "number",
                    // Match on boolean
                    "bool" => "boolean",
                    // Match on string types
                    s if s.contains("String") || s.contains("str") => "string",
                    // Default
                    _ => "string", // Default to string for unknown types
                };

                // TODO: Add description field from parameter doc comment?
                properties.insert(param_name.clone(), json!({ "type": json_type }));
            }
        }
    }

    let parameter_schema =
        json!({ "type": "object", "properties": properties, "required": required });
    let parameter_schema_str = parameter_schema.to_string();

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
        // Generate parse statements for each parameter
        let parse_stmts = param_names
            .iter()
            .zip(param_types.iter())
            .enumerate()
            .map(|(i, (name, ty))| {
                let var = Ident2::new(&format!("arg{}", i), Span::call_site());
                let idx = syn::Index::from(i);
                quote! {
                    let #var = match args[#idx].parse::<#ty>() {
                        Ok(v) => v,
                        Err(_) => return Err(format!(
                            "Failed to parse argument '{}' for parameter '{}'",
                            args[#idx], #name
                        ))
                    };
                }
            })
            .collect::<Vec<_>>();

        // Generate argument list for function call
        let call_args = (0..param_count)
            .map(|i| {
                let var = Ident2::new(&format!("arg{}", i), Span::call_site());
                quote! { #var }
            })
            .collect::<Vec<_>>();

        // Generate the function body
        quote! {
            |args: &[String]| {
                if args.len() != #param_count {
                    return Err(format!("Expected {} arguments, got {}", #param_count, args.len()));
                }

                // Parse each argument
                #(#parse_stmts)*

                // Call function with parsed arguments
                let result = #fn_ident(#(#call_args),*);
                Ok(result)
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
                parameter_schema: serde_json::from_str(#parameter_schema_str).unwrap_or(serde_json::Value::Null),
                function: #func_body,
            }
        }
    };

    expanded.into()
}
