use iwm_runtime_host::{RuntimeDiagnostic, RuntimeDiagnosticLevel, RuntimeHost};

use crate::RuntimeCore;

const MAX_DIAGNOSTICS: usize = 64;

impl RuntimeCore {
    pub(crate) fn record_diagnostic<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        level: RuntimeDiagnosticLevel,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        let diagnostic = RuntimeDiagnostic {
            level,
            code: code.into(),
            message: message.into(),
        };
        host.record(diagnostic.clone());
        if self.diagnostics.len() >= MAX_DIAGNOSTICS {
            self.diagnostics.remove(0);
        }
        self.diagnostics.push(diagnostic);
    }
}
