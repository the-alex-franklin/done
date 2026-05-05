use swc_core::{
    common::{sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS},
    ecma::{
        ast::{EsVersion, Pass, Program},
        codegen::{text_writer::JsWriter, Emitter},
        parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax},
        transforms::typescript::strip,
    },
};

pub fn transpile(ts_source: &str) -> anyhow::Result<String> {
    let cm: Lrc<SourceMap> = Default::default();

    GLOBALS.set(&Globals::default(), || -> anyhow::Result<String> {
        let fm = cm.new_source_file(
            Lrc::new(FileName::Anon),
            ts_source.to_string(),
        );

        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax::default()),
            EsVersion::Es2020,
            StringInput::from(&*fm),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser
            .parse_module()
            .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;

        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        let mut program = Program::Module(module);
        strip(unresolved_mark, top_level_mark).process(&mut program);
        let module = match program {
            Program::Module(m) => m,
            _ => unreachable!(),
        };

        let mut buf = Vec::new();
        {
            let mut emitter = Emitter {
                cfg: Default::default(),
                cm: cm.clone(),
                comments: None,
                wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };
            emitter
                .emit_module(&module)
                .map_err(|e| anyhow::anyhow!("Codegen error: {e}"))?;
        }

        String::from_utf8(buf).map_err(|e| anyhow::anyhow!("Non-UTF8 output: {e}"))
    })
}
