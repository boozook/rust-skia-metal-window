use std::mem;
use metal::*;
use foreign_types::ForeignType;
use foreign_types::ForeignTypeRef;
use objc::{rc::autoreleasepool, runtime::YES};
use cocoa::{appkit::NSView, base::id as cocoa_id};
use skia::ColorSpace;
use skia::ColorType;
use skia::Surface;
use skia::colors::WHITE;
use skia::gpu::DirectContext;
use skia::gpu::BackendRenderTarget;
use skia::gpu::SurfaceOrigin;
use skia::gpu::mtl::TextureInfo;
use winit_input_helper::WinitInputHelper;
use winit::platform::macos::WindowExtMacOS;
use winit::event::{Event, WindowEvent, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;


mod renderer;


fn main() {
	let events_loop = EventLoop::new();
	let mut input = WinitInputHelper::new();

	let size = winit::dpi::LogicalSize::new(1042, 1042);

	let window = winit::window::WindowBuilder::new().with_inner_size(size)
	                                                .with_title("Skia with Metal backend".to_string())
	                                                // .with_resizable(false)
	                                                .build(&events_loop)
	                                                .unwrap();

	let device = Device::system_default().expect("no device found");

	let layer = MetalLayer::new();
	layer.set_device(&device);
	layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
	// layer.set_presents_with_transaction(false);
	// layer.display_sync_enabled();

	layer.set_framebuffer_only(false);
	layer.set_opaque(true);

	layer.set_maximum_drawable_count(3);

	unsafe {
		let view = window.ns_view() as cocoa_id;
		view.setWantsLayer(YES);
		view.setLayer(mem::transmute(layer.as_ref()));
	};

	let draw_size = window.inner_size();
	layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));

	let command_queue = device.new_command_queue();


	let mut ctx = unsafe {
		DirectContext::new_metal(device.as_ptr() as *mut _, command_queue.as_ptr() as *mut _, None).expect("Unable to create direct context")
	};

	fn create_surface(ctx: &mut DirectContext, window: &Window, drawable: &MetalDrawableRef) -> Option<Surface> {
		println!("create_surface start");
		let draw_size = window.inner_size();
		let t_info = unsafe { TextureInfo::new(drawable.texture().as_ptr() as *const _) };
		let target = BackendRenderTarget::new_metal((draw_size.width as i32, draw_size.height as i32), 4, &t_info);

		// TODO: let color_type = layer.pixel_format();

		let surface = Surface::from_backend_render_target(
		                                                  ctx,
		                                                  &target,
		                                                  SurfaceOrigin::BottomLeft,
		                                                  ColorType::BGRA8888,
		                                                  ColorSpace::new_srgb(),
		                                                  None,
		);

		surface
	}


	let mut frame = 0;

	events_loop.run(move |event, _, control_flow| {
		           autoreleasepool(|| {
			           // *control_flow = ControlFlow::Poll;
			           *control_flow = ControlFlow::Wait;

			           match &event {
				           Event::WindowEvent { event, .. } => {
				              match event {
					              WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
				                 WindowEvent::Resized(size) => {
					                 println!("Resized: {:?}", size);
					                 layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
					                 // XXX: re-create / resize the surface
				                 },

				                 _ => (),
				              }
			              },
			              Event::MainEventsCleared => {
				              window.request_redraw();
			              },
			              Event::RedrawRequested(_) => {
				              println!("frame {}", frame);

				              if let Some(drawable) = layer.next_drawable() {
					              let surface = create_surface(&mut ctx, &window, drawable);

					              if let Some(mut surface) = surface {
						              // skia render:
						              let canvas = surface.canvas();
						              canvas.clear(WHITE);
						              renderer::render_frame(frame % 360, 12, 60, canvas);

						              surface.canvas().flush();
						              ctx.flush_and_submit();

						              //   let render_pass_descriptor = RenderPassDescriptor::new();
						              //   prepare_render_pass_descriptor(&render_pass_descriptor, drawable.texture());
						              let command_buffer = command_queue.new_command_buffer();
						              command_buffer.present_drawable(drawable);
						              command_buffer.commit();
					              } else {
						              println!("unable to create surface");
					              }
				              } else {
					              println!("frame skip, no drawable");
				              }
				              frame += 1;
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
