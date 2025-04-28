extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Span};
use quote::quote;
use serde_json::json;
use syn::{
    parse_macro_input, Expr, FnArg, GenericArgument, ItemFn, Lit, Meta, Pat, PathArguments, Type,
};

/// Helper function to deny reference types in parameters
fn deny_references(ty: &Type) -> Result<(), syn::Error> {
    if matches!(ty, Type::Reference(_)) {
        Err(syn::Error::new_spanned(
            ty,
            "reference types (`&T`) are not supported; use owned types like `String` or `Vec<T>`",
        ))
    } else {
        Ok(())
    }
}

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

    // Check if the function is async
    let is_async = sig.asyncness.is_some();

    // Get input parameters for parsing arguments
    let mut param_types = Vec::new();
    let mut param_names = Vec::new();
    let mut param_is_option = Vec::new(); // Track if param is Option<T>
    let mut param_defaults = Vec::new(); // Track default values from attributes

    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            // Check for reference types early
            if let Err(e) = deny_references(&pat_type.ty) {
                return e.to_compile_error().into();
            }

            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                param_names.push(param_name);

                // Check if the type is Option<T>
                let (is_option, inner_ty) = is_option_type(&pat_type.ty);
                param_is_option.push(is_option);
                param_types.push(if is_option {
                    inner_ty.unwrap()
                } else {
                    &pat_type.ty
                }); // Store inner type if Option

                // Parse #[default = ...] attribute if present (simplified parsing)
                let default_value = match find_default_attr(&pat_type.attrs) {
                    Ok(v) => v,
                    Err(e) => return e.to_compile_error().into(),
                };
                param_defaults.push(default_value);
            }
        }
    }

    // Build JSON Schema for parameters
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    // Need to iterate using indices to access is_option and defaults simultaneously
    for i in 0..param_names.len() {
        let param_name = &param_names[i];
        let param_type = param_types[i]; // This is the inner type for Option<T>
        let is_option = param_is_option[i];
        let default_value = &param_defaults[i];

        // Only add non-optional parameters to the required list
        if !is_option {
            required.push(json!(param_name.clone()));
        }

        // Convert the type to a string for schema generation
        let type_str = quote!(#param_type).to_string().replace(" ", "");
        let base_json_type = match type_str.as_str() {
            s if s.starts_with("u") || s.starts_with("i") || s == "usize" || s == "isize" => {
                "integer"
            }
            "f32" | "f64" => "number",
            "bool" => "boolean",
            s if s.contains("String") => "string", // Be more specific for String
            _ => "string",                         // Default, consider improving
        };

        let mut param_schema = serde_json::Map::new();
        if is_option {
            // Allow null for optional types
            param_schema.insert("type".to_string(), json!([base_json_type, "null"]));
        } else {
            param_schema.insert("type".to_string(), json!(base_json_type));
        }

        // Add default value to schema if present
        if let Some(default_lit) = default_value {
            // Attempt to convert syn::Lit to serde_json::Value
            let default_json_val = match default_lit {
                Lit::Str(s) => json!(s.value()),
                Lit::Int(i) => match i.base10_parse::<i64>() {
                    Ok(v) => json!(v),
                    Err(e) => return syn::Error::new_spanned(i, e).to_compile_error().into(),
                },
                Lit::Float(f) => match f.base10_parse::<f64>() {
                    Ok(v) => json!(v),
                    Err(e) => return syn::Error::new_spanned(f, e).to_compile_error().into(),
                },
                Lit::Bool(b) => json!(b.value),
                _ => {
                    return syn::Error::new_spanned(default_lit, "Unsupported default value type")
                        .to_compile_error()
                        .into()
                }
            };
            param_schema.insert("default".to_string(), default_json_val);
        }

        properties.insert(param_name.clone(), serde_json::Value::Object(param_schema));
    }

    let parameter_schema =
        json!({ "type": "object", "properties": properties, "required": required });
    let parameter_schema_str = parameter_schema.to_string();

    // Count the parameters directly
    let param_count = param_names.len();

    // Generate a constructor function for the Tool rather than using static initialization
    let metadata_fn = syn::Ident::new(&format!("__register_tool_{}", fn_name), fn_ident.span());

    // Generate the single async closure, wrapping sync functions if needed
    let func_body = {
        // Determine how many parameters are required (non-Option)
        let required_count = required.len();
        // Async length-check: allow between required_count and param_count
        let async_check_len_stmt = quote! {
            if args.len() < #required_count || args.len() > #param_count {
                return Box::pin(futures::future::ready(Err(
                    tool_calling::ToolError::BadArgs(format!(
                        "Expected between {} and {} arguments, got {}",
                        #required_count,
                        #param_count,
                        args.len()
                    ))
                )));
            }
        };
        // Sync length-check: same range
        let sync_check_len_stmt = quote! {
            if args.len() < #required_count || args.len() > #param_count {
                return Err(tool_calling::ToolError::BadArgs(format!(
                    "Expected between {} and {} arguments, got {}",
                    #required_count,
                    #param_count,
                    args.len()
                )));
            }
        };

        let parse_and_call_logic = if param_count == 0 {
            if is_async {
                quote! {
                    match #fn_ident().await {
                        result => Ok(result),
                        // TODO: Consider capturing panics or mapping errors if the function returns Result
                        // Err(e) => Err(tool_calling::ToolError::Execution(e.to_string())),
                    }
                }
            } else {
                quote! {
                    // No need to capture panics explicitly for sync, wrap_sync handles the Result
                    Ok(#fn_ident())
                }
            }
        } else {
            // Generate parse statements for each parameter using ToolError, handling Option and defaults
            let parse_stmts = param_names
                .iter()
                .zip(param_types.iter()) // Use potentially inner type
                .zip(param_is_option.iter())
                .zip(param_defaults.iter())
                .enumerate()
                .map(|(i, (((name, ty), is_option), default_value))| {
                    let var = Ident2::new(&format!("arg{}", i), Span::call_site());
                    let idx = syn::Index::from(i);

                    let parse_expr = quote! {
                         owned_args[#idx].parse::<#ty>()
                            .map_err(|_| tool_calling::ToolError::BadArgs(format!(
                                "Failed to parse argument '{}' for parameter '{}'",
                                owned_args[#idx], #name
                            )))
                    };

                    if *is_option {
                        let default_branch = match default_value {
                            Some(lit) => quote! { Some(#lit) }, // Use the literal directly if default provided
                            None => quote! { None },            // No default means None for Option
                        };
                        quote! {
                            let #var: Option<#ty> = match owned_args.get(#idx) {
                                Some(s) => Some(#parse_expr?),
                                None => #default_branch, // Use default or None
                            };
                        }
                    } else {
                        // Non-optional: Must parse or fail (unless default exists? No, schema validation ensures presence if no default)
                        quote! {
                            // Parameter is required, so owned_args[#idx] should exist due to schema validation.
                            let #var = #parse_expr?;
                        }
                    }
                })
                .collect::<Vec<_>>();

            // Generate argument list for function call (variables now include options)
            let call_args = (0..param_count)
                .map(|i| {
                    let var = Ident2::new(&format!("arg{}", i), Span::call_site());
                    quote! { #var }
                })
                .collect::<Vec<_>>();

            if is_async {
                quote! {
                    // Parse each argument
                    #(#parse_stmts)*
                    // Call function with parsed arguments
                    match #fn_ident(#(#call_args),*).await {
                         result => Ok(result),
                        // TODO: Capture panics or map errors
                        // Err(e) => Err(tool_calling::ToolError::Execution(e.to_string())),
                    }
                }
            } else {
                quote! {
                    // Parse each argument
                    #(#parse_stmts)*
                    // Call function with parsed arguments
                    Ok(#fn_ident(#(#call_args),*)) // Wrap result in Ok for wrap_sync
                }
            }
        };

        // The final function body expression for ToolFn
        if is_async {
            quote! {
                 tool_calling::ToolFn::Async(Box::new(|args: &[String]| {
                     // Perform checks and clone args *before* creating the BoxFuture
                     #async_check_len_stmt // Use async check
                     let owned_args = args.to_vec();
                     Box::pin(async move {
                        #parse_and_call_logic
                    })
                 }))
            }
        } else {
            // Wrap the synchronous logic using the helper
            quote! {
                 tool_calling::ToolFn::Async(tool_calling::wrap_sync(
                     // Use Arc::new instead of Box::new
                     std::sync::Arc::new(|args: &[String]| {
                         #sync_check_len_stmt
                         // Clone args into a Vec for parsing logic
                         let owned_args = args.to_vec();
                         #parse_and_call_logic // This uses owned_args Vec
                     }) as std::sync::Arc<dyn Fn(&[String]) -> Result<String, tool_calling::ToolError> + Send + Sync>
                 ))
            }
        }
    };

    let expanded = quote! {
        #input_fn

        #[doc(hidden)]
        #[linkme::distributed_slice(tool_calling::TOOL_FACTORIES)]
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

/// Checks if a type is Option<T> and returns the inner type T if so.
fn is_option_type(ty: &Type) -> (bool, Option<&Type>) {
    if let Type::Path(type_path) = ty {
        if type_path.qself.is_none() {
            let path = &type_path.path;
            // Check if the path ends with "Option"
            if let Some(last_segment) = path.segments.last() {
                if last_segment.ident == "Option" {
                    // Check if it has angle bracketed arguments like <T>
                    if let PathArguments::AngleBracketed(params) = &last_segment.arguments {
                        // Check if there is exactly one generic argument
                        if params.args.len() == 1 {
                            // Get the first argument
                            if let Some(GenericArgument::Type(inner_ty)) = params.args.first() {
                                return (true, Some(inner_ty));
                            }
                        }
                    }
                }
            }
        }
    }
    (false, None)
}

/// Finds a `#[default = lit]` attribute on a parameter.
fn find_default_attr(attrs: &[syn::Attribute]) -> Result<Option<Lit>, syn::Error> {
    for attr in attrs {
        if attr.path().is_ident("default") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(expr_lit) = &nv.value {
                    return Ok(Some(expr_lit.lit.clone()));
                }
            }
            return Err(syn::Error::new_spanned(
                attr,
                "Expected `#[default = <literal>]`",
            ));
        }
    }
    Ok(None)
}

// Add a passthrough attribute macro for `default` on parameters
#[proc_macro_attribute]
pub fn default(_args: TokenStream, item: TokenStream) -> TokenStream {
    // Simply return the item unchanged
    item
}
