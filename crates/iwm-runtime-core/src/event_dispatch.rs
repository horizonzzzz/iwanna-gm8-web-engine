#[derive(Clone)]
pub(crate) enum RuntimeEventSelector {
    Alarm(u32),
    Keyboard(u16),
}

use crate::RuntimePackage;

pub(crate) fn object_event_block_ids(
    package: &RuntimePackage,
    object_id: usize,
    selector: RuntimeEventSelector,
) -> Vec<String> {
    let wanted = match selector {
        RuntimeEventSelector::Alarm(slot) => format!("alarm:{slot}"),
        RuntimeEventSelector::Keyboard(key) => {
            format!("keyboard:{}", format_key_name(key))
        }
    };

    package
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .into_iter()
        .flat_map(|object| object.events.iter())
        .filter(|event| event.event_tag == wanted)
        .map(|event| event.block_id.clone())
        .collect()
}

fn format_key_name(sub_event: u16) -> String {
    let key = sub_event as u8 as char;
    if key.is_ascii_alphanumeric() {
        key.to_ascii_lowercase().to_string()
    } else {
        format!("0x{:02x}", sub_event as u8)
    }
}
