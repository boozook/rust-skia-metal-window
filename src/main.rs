use metal::*;
use cocoa::{appkit::NSView, base::id as cocoa_id};
use foreign_types::ForeignType;
use objc::{rc::autoreleasepool, runtime::YES};
use skia::ColorSpace;
use skia::ColorType;
use skia::Surface;
use skia::colors::WHITE;
use skia::gpu::SurfaceOrigin;
use winit_input_helper::WinitInputHelper;
use winit::platform::macos::WindowExtMacOS;
use winit::event::{Event, WindowEvent, VirtualKeyCode};
use winit::event_loop::ControlFlow;


mod renderer;


fn main() {
	let events_loop = winit::event_loop::EventLoop::new();
	let mut input = WinitInputHelper::new();

	let size = winit::dpi::LogicalSize::new(1042, 1042);

	let window = winit::window::WindowBuilder::new().with_inner_size(size)
	                                                .with_title("Skia with Metal backend".to_string())
	                                                .build(&events_loop)
	                                                .unwrap();

	let device = Device::system_default().expect("no device found");

	let layer = MetalLayer::new();
	layer.set_device(&device);
	layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
	layer.display_sync_enabled();
	layer.set_framebuffer_only(false);
	layer.set_opaque(true);

	unsafe {
		let view = window.ns_view() as cocoa_id;
		view.setWantsLayer(YES);
		view.setLayer(layer.as_ptr() as *mut _ as cocoa_id);
	};

	let draw_size = window.inner_size();
	layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));

	let command_queue = device.new_command_queue();

	let (mut ctx, (mut surface, drawable)) = {
		let mut ctx = unsafe {
			skia::gpu::DirectContext::new_metal(
			                                    device.as_ptr() as *mut _,
			                                    command_queue.as_ptr() as *mut _,
			                                    None,
			).unwrap()
		};

		let surface = Surface::from_ca_metal_layer(
		                                           &mut ctx,
		                                           layer.as_ptr() as *mut _,
		                                           SurfaceOrigin::BottomLeft,
		                                           Some(4),
		                                           ColorType::BGRA8888,
		                                           ColorSpace::new_srgb(),
		                                           None,
		).expect("Unable to create surface");

		(ctx, surface)
	};


	let mut frame = 0;

	events_loop.run(move |event, _, control_flow| {
		           autoreleasepool(|| {
			           *control_flow = ControlFlow::Poll;

			           match &event {
				           Event::WindowEvent { event, .. } => {
				              match event {
					              WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
				                 WindowEvent::Resized(size) => {
					                 layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
					                 // TODO: re-create / resize the surface
				                 },

				                 _ => (),
				              }
			              },
			              Event::MainEventsCleared => {
				              window.request_redraw();
			              },
			              Event::RedrawRequested(_) => {
				              {
					              let canvas = surface.canvas();
					              canvas.clear(WHITE);
					              renderer::render_frame(frame % 360, 12, 60, canvas);
				              }
				              surface.canvas().flush();
				              ctx.flush_and_submit();
			              },

			              _ => {},
			           }


			           // Handle input events
			           if input.update(&event) {
				           // Close events
				           let cmd_q = (input.key_held(VirtualKeyCode::LWin) || input.key_held(VirtualKeyCode::RWin)) &&
				                       (input.key_pressed(VirtualKeyCode::Q) ||
				                        input.key_pressed(VirtualKeyCode::W));
				           if cmd_q || input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
					           *control_flow = ControlFlow::Exit;
					           return;
				           }

				           if input.key_pressed(VirtualKeyCode::Space) {
					           frame += 1;
					           window.request_redraw();
				           }
			           }
		           });
	           });
}
