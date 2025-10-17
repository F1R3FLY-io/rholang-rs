use std::{fs, path::PathBuf};

use librho::sem::{
    EnclosureAnalysisPass, ResolverPass, SemanticDb, diagnostics::UnusedVarsPass,
    pipeline::Pipeline,
};
use rholang_parser::RholangParser;

fn main() {
    divan::main();
}

#[divan::bench(args = each_rho_file())]
fn sem_anal(bencher: divan::Bencher, arg: &PathBuf) {
    let code = fs::read_to_string(arg).expect("expected a readable file");
    let parser = RholangParser::new();
    let parsed = parser.parse(&code).expect("expected valid Rholang code");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .expect("expected Tokio runtime");

    bencher.bench_local(|| {
        let mut db = SemanticDb::new();

        let pipeline = parsed
            .iter()
            .fold(Pipeline::new(), |pipeline, ast| {
                let root = db.build_index(ast);
                pipeline
                    .add_fact(ResolverPass::new(root))
                    .add_fact(EnclosureAnalysisPass::new(root))
            })
            .add_diagnostic(UnusedVarsPass);

        runtime.block_on(pipeline.run(&mut db));

        assert!(
            !db.has_errors(),
            "Benchamrk finished with errors:\n{:?}",
            db.diagnostics()
        );
        divan::black_box_drop(db);
    });
}

fn each_rho_file() -> impl Iterator<Item = PathBuf> {
    fs::read_dir("benches")
        .expect("expected benches directory to exist")
        .map(|dir_entry_or_error| dir_entry_or_error.unwrap())
        .filter_map(|dir_entry| {
            let path = dir_entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rho") {
                Some(path)
            } else {
                None
            }
        })
}
