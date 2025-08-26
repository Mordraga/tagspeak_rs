#![cfg(feature = "linux")]

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("TagSpeak Setup")
        .build(&event_loop)
        .expect("window");

    event_loop.run(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if matches!(
            event,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
        ) {
            *control_flow = ControlFlow::Exit;
        }
    });
}
