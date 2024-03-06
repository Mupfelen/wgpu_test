struct KeyboardInputHandler {
    
}

impl KeyboardInputHandler {
    fn new() -> KeyboardInputHandler {
        KeyboardInputHandler {
            
        }
    }
    
    fn input(&self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input,
                device_id,
                is_synthetic
            } => {
                if input.state == ElementState::Pressed {
                    println!("Key pressed: {:?}", input.virtual_keycode);
                }
            }
            _ => {}
        }
        false
    }
}
