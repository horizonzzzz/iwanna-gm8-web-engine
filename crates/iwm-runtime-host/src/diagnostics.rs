use crate::{RuntimeDiagnostic, RuntimeDiagnosticsHost};

const MAX_DIAGNOSTICS: usize = 64;

#[derive(Debug, Default)]
pub struct VecDiagnosticsHost {
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

impl RuntimeDiagnosticsHost for VecDiagnosticsHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        if self.diagnostics.len() >= MAX_DIAGNOSTICS {
            self.diagnostics.remove(0);
        }
        self.diagnostics.push(diagnostic);
    }
}
