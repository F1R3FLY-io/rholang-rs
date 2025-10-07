use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, LitStr, Pat, PatType, Type, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn test_rholang_code(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute input
    let code_arg = parse_macro_input!(attr as LitStr);
    let code_str = code_arg.value();

    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    let func_block = &func.block;
    let inputs = &func.sig.inputs;

    // Check signature validity
    if inputs.len() != 2 {
        return syn::Error::new(
            func.sig.span(),
            "expected exactly two arguments: (proc(s), db)",
        )
        .to_compile_error()
        .into();
    }

    // Extract argument (ident, type)
    fn extract_ident_and_ty(arg: &FnArg) -> Option<(&syn::Ident, &Type)> {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(ident) = pat.as_ref() {
                return Some((&ident.ident, ty.as_ref()));
            }
        }
        None
    }

    // Parse both arguments
    let arg1 = &inputs[0];
    let arg2 = &inputs[1];

    if let Some((name1, ty1)) = extract_ident_and_ty(arg1) {
        if let Some((name2, ty2)) = extract_ident_and_ty(arg2) {
            // Identify which args correspond to which roles
            let class1 = classify_type(ty1);
            let class2 = classify_type(ty2);

            // --- Validate classification ---
            let valid = class1.is_some() && class2.is_some();

            if !valid {
                return syn::Error::new_spanned(
                    &func.sig.inputs,
                    "expected (AnnProc/&[AnnProc]) and (&SemanticDb/&mut SemanticDb)",
                )
                .to_compile_error()
                .into();
            }

            // --- Hygiene: create unique identifiers ---
            let parser_ident = format_ident!("__parser_{}", func_name);
            let parsed_ident = format_ident!("__parsed_{}", func_name);
            let db_ident = format_ident!("__db_{}", func_name);
            let procs_ident = format_ident!("__procs_{}", func_name);

            // --- Determine how to bind the arguments ---
            let bind_procs = class1
                .unwrap()
                .bind_argument(name1, &db_ident, &procs_ident);
            let bind_db = class2
                .unwrap()
                .bind_argument(name2, &db_ident, &procs_ident);

            // Build the expanded test
            let expanded = quote! {
                #[test]
                fn #func_name() {
                    let code = #code_str;

                    let #parser_ident = rholang_parser::RholangParser::new();
                    let #parsed_ident = #parser_ident.parse(code);

                    match #parsed_ident {
                        validated::Validated::Good(#procs_ident) => {
                            let mut #db_ident = SemanticDb::new();
                            for proc in &#procs_ident {
                                #db_ident.build_index(proc);
                            }

                            {
                                #bind_procs
                                #bind_db

                                #func_block
                            }
                        }
                        validated::Validated::Fail(nevec) => panic!(
                            "Test failed because the provided rholang code could not be parsed correctly. The errors are:\n{nevec:#?}"
                        ),
                    }

                }
            };

            expanded.into()
        } else {
            return syn::Error::new(
                arg2.span(),
                "unsupported argument pattern: expected plain literal",
            )
            .to_compile_error()
            .into();
        }
    } else {
        return syn::Error::new(
            arg1.span(),
            "unsupported argument pattern: expected plain literal",
        )
        .to_compile_error()
        .into();
    }
}

fn classify_type(ty: &Type) -> Option<Classification> {
    fn path_contains(path: &syn::Path, name: &str) -> bool {
        path.segments.last().map_or(false, |s| s.ident == name)
    }
    match ty {
        Type::Reference(r) => match r.elem.as_ref() {
            Type::Slice(_) => Some(Classification::ProcRefSlice),
            Type::Path(p) if path_contains(&p.path, "SemanticDb") => {
                if r.mutability.is_some() {
                    Some(Classification::MutSemDbRef)
                } else {
                    Some(Classification::SemDbRef)
                }
            }
            Type::Path(_) => Some(Classification::ProcRef),
            _ => None,
        },
        Type::Path(p) if path_contains(&p.path, "ProcRef") => Some(Classification::ProcRef),
        _ => None,
    }
}

enum Classification {
    SemDbRef,
    MutSemDbRef,
    ProcRefSlice,
    ProcRef,
}

impl Classification {
    fn bind_argument(
        self,
        to: &syn::Ident,
        db: &syn::Ident,
        procs: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        match self {
            Classification::ProcRefSlice => quote! { let #to = &#procs; },
            Classification::ProcRef => {
                quote! { let #to = if #procs.is_empty() { panic!("Parser did not produce any output") } else { &#procs[0] }; }
            }
            Classification::SemDbRef => quote! { let #to = &#db; },
            Classification::MutSemDbRef => quote! { let #to = &mut #db; },
        }
    }
}
