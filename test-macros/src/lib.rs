use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat, PatType, Path, Result, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

struct TestRholangCodeArgs {
    code: LitStr,
    pipeline: Option<Path>,
}

impl TestRholangCodeArgs {
    fn generate_test_setup(&self, procs: &syn::Ident, db: &syn::Ident) -> proc_macro2::TokenStream {
        match &self.pipeline {
            Some(pipeline_func) => {
                let pipeline = syn::Ident::new("pipeline", Span::mixed_site());
                quote! {
                    let #pipeline = #pipeline_func(#procs.iter().map(|proc| #db.build_index(proc)));
                    println!("Running the pipeline:\n{}", #pipeline.describe());
                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(#pipeline.run(&mut #db));
                }
            }
            None => quote! {
                for proc in &#procs {
                    #db.build_index(proc);
                }
            },
        }
    }
}

impl Parse for TestRholangCodeArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        // First, the required string literal
        let code: LitStr = input.parse()?;
        let mut pipeline: Option<Path> = None;

        // Check for optional trailing arguments
        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let ident: syn::Ident = input.parse()?; // should be 'pipeline'
            if ident == "pipeline" {
                let _eq: Token![=] = input.parse()?;
                let func: Path = input.parse()?;
                pipeline = Some(func);
            } else {
                return Err(syn::Error::new_spanned(ident, "expected `pipeline = ...`"));
            }
        }

        Ok(Self { code, pipeline })
    }
}

#[proc_macro_attribute]
pub fn test_rholang_code(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute input
    let args = parse_macro_input!(attr as TestRholangCodeArgs);
    let code_str = args.code.value();

    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    let func_block = &func.block;
    let generics = &func.sig.generics;
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
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg
            && let Pat::Ident(ident) = pat.as_ref()
        {
            return Some((&ident.ident, ty.as_ref()));
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
            let db_ident = syn::Ident::new("db", Span::mixed_site());
            let procs_ident = syn::Ident::new("procs", Span::mixed_site());
            let inner_func_ident = format_ident!("_{}", func_name, span = Span::mixed_site());

            // --- Determine how to bind the arguments ---
            let bind_1 = class1.unwrap().bind_argument(&db_ident, &procs_ident);
            let bind_2 = class2.unwrap().bind_argument(&db_ident, &procs_ident);

            // Build the expanded test
            let test_setup = args.generate_test_setup(&procs_ident, &db_ident);
            let expanded = quote! {
                #[test]
                fn #func_name() {
                    let code = #code_str;

                    let parser = rholang_parser::RholangParser::new();
                    let parsed = parser.parse(code);

                    match parsed {
                        validated::Validated::Good(#procs_ident) => {
                            let mut #db_ident = SemanticDb::new();
                            #test_setup

                            fn #inner_func_ident #generics(#name1: #ty1, #name2: #ty2) {
                                #func_block
                            }

                            #inner_func_ident(#bind_1, #bind_2);
                        }
                        validated::Validated::Fail(nevec) => panic!(
                            "Test failed: invalid rholang code.\nErrors:\n{nevec:#?}"
                        ),
                    }

                }
            };

            expanded.into()
        } else {
            syn::Error::new(arg2.span(), "expected simple identifier arguments")
                .to_compile_error()
                .into()
        }
    } else {
        syn::Error::new(arg1.span(), "expected simple identifier arguments")
            .to_compile_error()
            .into()
    }
}

fn classify_type(ty: &Type) -> Option<Classification> {
    fn path_contains(path: &syn::Path, name: &str) -> bool {
        path.segments.last().is_some_and(|s| s.ident == name)
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
    fn bind_argument(self, db: &syn::Ident, procs: &syn::Ident) -> proc_macro2::TokenStream {
        match self {
            Classification::ProcRefSlice => quote! { &#procs },
            Classification::ProcRef => {
                quote! { { if #procs.is_empty() { panic!("Parser did not produce any output") } else { &#procs[0] } } }
            }
            Classification::SemDbRef => quote! { &#db },
            Classification::MutSemDbRef => quote! { &mut #db },
        }
    }
}
