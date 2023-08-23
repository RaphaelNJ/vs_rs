pub trait GetName {
    fn get_name(&self) -> String;
}

pub fn uniquify_name(input_name: String, vec: &Vec<impl GetName>) -> String {
    let mut times = 0;
    let name = input_name.replace(" ", "_");
    loop {
        let x = 'x: {
            for obj in vec.iter() {
                if times == 0 {
                    if obj.get_name() == name {
                        times += 1;
                        break 'x true;
                    }
                } else {
                    if format!("{}_{}", name, times) == obj.get_name() {
                        times += 1;
                        break 'x true;
                    }
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
