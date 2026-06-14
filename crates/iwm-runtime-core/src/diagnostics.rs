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

pub(crate) fn record_execution_trace<H: RuntimeHost>(
    host: &mut H,
    diagnostics: &mut Vec<RuntimeDiagnostic>,
    room_id: usize,
    tick: u64,
    instance: &crate::RuntimeInstance,
    block_id: &str,
    event_tag: &str,
) {
    if event_tag == "collision" {
        return;
    }

    let diagnostic = RuntimeDiagnostic {
        level: RuntimeDiagnosticLevel::Info,
        code: "runtime-exec-block-trace".into(),
        message: format!(
            "room={} tick={} block_id={} object={} event_tag={} runtime_id={}",
            room_id, tick, block_id, instance.object_name, event_tag, instance.runtime_id
        ),
    };
    host.record(diagnostic.clone());
    if diagnostics.len() >= MAX_DIAGNOSTICS {
        diagnostics.remove(0);
    }
    diagnostics.push(diagnostic);
}
