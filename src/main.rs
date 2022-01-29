use wgpu_utils::framework::{State, Framework};

#[macro_use]
extern crate more_asserts;

extern crate nalgebra_glm as glm;
extern crate naga;

mod wgpu_utils;

struct WinState{

}

impl State for WinState{
    fn new(app: &mut wgpu_utils::framework::AppState) -> Self {
        Self{}
    }

    fn render(&mut self, app: &mut wgpu_utils::framework::AppState, control_flow: &mut winit::event_loop::ControlFlow) -> Result<(), wgpu::SurfaceError> {
        let output = app.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Render Encoder"),
        });




        app.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    let framework = Framework::<WinState>::new([800, 600]).run();
}
