use swc_core::common::sync::Lrc;
use swc_core::common::{
    errors::{ColorConfig, Handler},
    FileName, SourceMap, Spanned,
};
use swc_core::ecma::ast::{FnDecl, FnExpr, Function, Module};
use swc_core::ecma::parser::{lexer::Lexer, Capturing, Parser, StringInput, Syntax};
use swc_core::ecma::visit::{Visit, VisitWith};

fn parse() {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    // Real usage
    // let fm = cm
    //     .load_file(Path::new("test.js"))
    //     .expect("failed to load test.js");

    let fm = cm.new_source_file(
        FileName::Custom("test.js".into()).into(),
        r#"function foo() {
            function bar() {
            }
        }"#
        .into(),
    );

    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let capturing = Capturing::new(lexer);

    let mut parser = Parser::new_from(capturing);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_module()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("Failed to parse module.");

    Shower { handler: &handler }.visit_module(&module);

    dbg!(parser.input().take());
}
struct Shower<'a> {
    handler: &'a Handler,
}

impl Shower<'_> {
    fn show(&self, name: &str, node: &dyn Spanned) {
        let span = node.span();
        dbg!(&name, &span);

        // self.handler.struct_span_err(span, name).emit();
    }
}

impl Visit for Shower<'_> {
    fn visit_fn_decl(&mut self, n: &FnDecl) {
        dbg!(&n);
        self.show("FnDecl", n);
        n.visit_children_with(self)
    }
    fn visit_fn_expr(&mut self, n: &FnExpr) {
        dbg!(&n);
        self.show("FnExpr", n);
        n.visit_children_with(self)
    }

    fn visit_function(&mut self, n: &Function) {
        dbg!(&n);
        self.show("Function", n);
        n.visit_children_with(self)
    }

    fn visit_module(&mut self, n: &Module) {
        self.show("Module", n);
        n.visit_children_with(self)
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        parse();
    }
}
