use crate::functions;
use crate::app;

pub trait GetName {
    fn get_name(&self) -> String;
}

fn uniquify_name_closure<F>(input_name: String, mut condition: F) -> String
where
    F: FnMut(&str) -> bool,
{
    let mut times = 0;
    let name = input_name.replace(" ", "_");

    loop {
        let x = 'x: {
            if times == 0 {
                if condition(&name) {
                    times += 1;
                    break 'x true;
                }
            } else {
                if condition(&format!("{}_{}", name, times)) {
                    times += 1;
                    break 'x true;
                }
            }
            false
        };

        if !x {
            if times == 0 {
                return name;
            }
            return format!("{}_{}", name, times);
        }
    }
}

pub fn uniquify_name(input_name: String, vec: &Vec<impl GetName>) -> String {
    uniquify_name_closure(input_name, |name| {
        vec.iter().any(|obj| obj.get_name() == name)
    })
}

pub fn uniquify_name_slot(
    input_name: String,
    slot: &slotmap::SlotMap<functions::FunctionId, app::GraphFunction>,
) -> String {
    uniquify_name_closure(input_name, |name| {
        slot.values().any(|obj| obj.get_name() == name)
    })
}
