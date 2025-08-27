#[cfg(target_os = "linux")]
use winit::event::{Event, WindowEvent};
#[cfg(target_os = "linux")]
use winit::event_loop::EventLoop;
#[cfg(target_os = "linux")]
use winit::window::WindowBuilder;

#[cfg(target_os = "linux")]
fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let _window = WindowBuilder::new()
        .with_title("TagSpeak Setup")
        .build(&event_loop)
        .expect("window");

    event_loop
        .run(|event, elwt| {
            if matches!(
                event,
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                }
            ) {
                elwt.exit();
            }
        })
        .expect("event loop run");
}

#[cfg(not(target_os = "linux"))]
fn main() {}
