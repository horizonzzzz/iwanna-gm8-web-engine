use crate::{ExternalSignature, ExternalValue, RuntimeExternalHost, RuntimeHostError};

#[derive(Debug, Default)]
pub struct RejectingExternalHost {
    pub attempted_definitions: Vec<ExternalSignature>,
}

impl RuntimeExternalHost for RejectingExternalHost {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError> {
        self.attempted_definitions.push(signature.clone());
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for {}!{}",
            signature.library, signature.symbol
        )))
    }

    fn call(
        &mut self,
        handle: u32,
        _args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError> {
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for handle {}",
            handle
        )))
    }

    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError> {
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for library {}",
            library
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeHostErrorKind;

    #[test]
    fn rejecting_external_host_is_explicit() {
        let mut externals = RejectingExternalHost::default();
        let error = externals
            .define(ExternalSignature {
                library: "gmfmodsimple.dll".into(),
                symbol: "FMODSoundAdd".into(),
                arg_count: 2,
            })
            .unwrap_err();

        assert_eq!(error.kind(), RuntimeHostErrorKind::Unsupported);
        assert_eq!(externals.attempted_definitions.len(), 1);
    }
}
