use crate::{RuntimeDiagnostic, RuntimeDiagnosticsHost};

#[derive(Debug, Default)]
pub struct VecDiagnosticsHost {
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

impl RuntimeDiagnosticsHost for VecDiagnosticsHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}
