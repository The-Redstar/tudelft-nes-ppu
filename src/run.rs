use crate::cpu::Cpu;
use crate::screen::{ButtonName, Message, Screen, ScreenWriter, ScreenReader};
use crate::{Mirroring, Ppu, CPU_FREQ, HEIGHT, WIDTH};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, thread};
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn run_ppu<CPU: Cpu>(
    mirroring: Mirroring,
    cpu: &mut CPU,
    writer: &mut ScreenWriter,
    max_cycles: Option<usize>,
) -> Result<(), CPU::TickError> {
    const ITER_PER_CYCLE: usize = 1000;
    let mut ppu = Ppu::new(mirroring);

    let mut busy_time = Duration::default();
    let mut cycles = 0;
    let mut last_tick = Instant::now();

    let mut px = 0;
    let mut py = 0;


    loop {
        for iteration in 0..ITER_PER_CYCLE {
            if let ScreenWriter::Real {
                control_rx: buttons_rx,
                screen,
                ..
            } = writer
            {
                while let Ok(msg) = buttons_rx.try_recv() {
                    match msg {
                        Message::Button(name, pressed) => match name {
                            ButtonName::A1 => {
                                ppu.buttons.a1 = pressed;
                            }
                            ButtonName::B1 => {
                                ppu.buttons.b1 = pressed;
                            }
                            ButtonName::Up1 => {
                                ppu.buttons.up1 = pressed;
                            }
                            ButtonName::Down1 => {
                                ppu.buttons.down1 = pressed;
                            }
                            ButtonName::Left1 => {
                                ppu.buttons.left1 = pressed;
                            }
                            ButtonName::Right1 => {
                                ppu.buttons.right1 = pressed;
                            }
                            ButtonName::Start1 => {
                                ppu.buttons.start1 = pressed;
                            }
                            ButtonName::Select1 => {
                                ppu.buttons.select1 = pressed;
                            }
                            ButtonName::A2 => {
                                ppu.buttons.a2 = pressed;
                            }
                            ButtonName::B2 => {
                                ppu.buttons.b2 = pressed;
                            }
                            ButtonName::Up2 => {
                                ppu.buttons.up2 = pressed;
                            }
                            ButtonName::Down2 => {
                                ppu.buttons.down2 = pressed;
                            }
                            ButtonName::Left2 => {
                                ppu.buttons.left2 = pressed;
                            }
                            ButtonName::Right2 => {
                                ppu.buttons.right2 = pressed;
                            }
                            ButtonName::Start2 => {
                                ppu.buttons.start2 = pressed;
                            }
                            ButtonName::Select2 => {
                                ppu.buttons.select2 = pressed;
                            }
                        },
                        Message::Pause(true) => {
                            while let Message::Pause(true) =
                                buttons_rx.recv().expect("sender closed")
                            {
                            }
                            // skip over previous iterations
                            last_tick = Instant::now();
                        }
                        Message::Pause(false) => {}
                        Message::PixelPointed(posx,posy) => {
                            let reader = screen.0.as_ref();
                            if let ScreenReader::Real{window, ..}= reader {
                                //0: take the position
                                //1: take the screen size
                                let screensize = window.inner_size();
                                //2: compute relative screen dimensions
                                let (relx,rely) = (posx / screensize.width as f64, posy / screensize.height as f64);
                                //3: compute pointed pixel coordinates
                                (px,py) = ((WIDTH as f64 * relx) as i32,(HEIGHT as f64 * rely) as i32);
                                px=px.min(0).max(WIDTH as i32-1);
                                py=py.min(0).max(HEIGHT as i32-1);
                            }
                            
                        }
                    }
                }
            }

            if let Err(e) = cpu.tick(&mut ppu) {
                log::warn!("cpu stopped");
                return Err(e);
            }

            if iteration == 0 {
                println!("mouse coordinates: {},{}",px,py);
            }

            for _ in 0..3 {
                ppu.update(cpu, writer);

                // get color of pixel pointed to by cursor
                if let ScreenWriter::Real {
                    screen,
                    ..
                } = writer {
                    if let ScreenReader::Real{ pixels, .. } = &*screen.0 {
                        ppu.pointed_pixel[..2].clone_from_slice(
                            &pixels
                            .lock()
                            .expect("Failed to lock")
                            .frame_mut()
                            [(4 * (py as usize * WIDTH as usize + px as usize))..(4 * (py as usize * WIDTH as usize + px as usize)+3)]
                        );
                    }
                }
                
            }
        }

        cycles += ITER_PER_CYCLE;

        if let Some(max_cycles) = max_cycles {
            if cycles > max_cycles {
                break Ok(());
            }
        }

        let now = Instant::now();
        busy_time += now.duration_since(last_tick);

        let expected_time_spent = Duration::from_secs_f64((1.0 / CPU_FREQ) * cycles as f64);

        if expected_time_spent > busy_time {
            thread::sleep(expected_time_spent - busy_time);
        } else if cycles % 1000 == 0
            && (busy_time - expected_time_spent) > Duration::from_secs_f64(0.2)
        {
            println!(
                "emulation behind by {:?}. trying to catch up...",
                busy_time - expected_time_spent
            );
        }

        last_tick = now;
    }
}

/// Like [`run_cpu_headless`], but takes a cycle limit after which the function returns.
pub fn run_cpu_headless_for<CPU>(
    cpu: &mut CPU,
    mirroring: Mirroring,
    cycle_limit: usize,
) -> Result<(), CPU::TickError>
where
    CPU: Cpu + 'static,
{
    let (_, mut writer) = Screen::dummy();

    run_ppu(mirroring, cpu, &mut writer, Some(cycle_limit))
}

/// Runs the cpu as if connected to a PPU, but doesn't actually open
/// a window. This can be useful in tests.
pub fn run_cpu_headless<CPU>(cpu: &mut CPU, mirroring: Mirroring) -> Result<(), CPU::TickError>
where
    CPU: Cpu + 'static,
{
    let (_, mut writer) = Screen::dummy();

    run_ppu(mirroring, cpu, &mut writer, None)
}

/// Runs the cpu with the ppu. Takes ownership of the cpu, creates
/// a PPU instance, and runs the tick function at the correct rate.
///
/// This function *has to be called from the main thread*. This means it will not
/// work from unit tests. Use [`run_cpu_headless`] there.
///
/// # Panics
/// [`run_cpu`] can panic when the `cpu` returns an Error
pub fn run_cpu<CPU>(mut cpu: CPU, mirroring: Mirroring)
where
    CPU: Cpu + Send + 'static,
{
    env::set_var("WAYLAND_DISPLAY", "wayland-1");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("NES")
        .build(&event_loop)
        .expect("failed to create window");

    //let window_size = window.inner_size();

    //println!("Window size: {window_size:?}");
    //modification for duck hunt
    //force canvas to take up full window
    window.set_inner_size(PhysicalSize::new(WIDTH*2,HEIGHT*2));
    let window_size = window.inner_size();

    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).expect("failed to create surface");

    let (mut screen, mut writer, control_tx) = Screen::new(pixels, window);

    let handle = Arc::new(Mutex::new(Some(thread::spawn(move || {
        match run_ppu(mirroring, &mut cpu, &mut writer, None) {
            Ok(_) => unreachable!(),
            Err(e) => {
                panic!("cpu implementation returned an error: {e}")
            }
        }
    }))));

    let mut last = Instant::now();
    let wait_time = Duration::from_secs_f64(1.0 / 60.0);

    event_loop.run(move |event, _, control_flow| {
        #[allow(clippy::single_match)]
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                return;
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(f),
                ..
            } => {
                control_tx.send(Message::Pause(!f)).expect("failed to send");
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                /* DUCK HUNT ADDITION */
                
                control_tx
                    .send(Message::PixelPointed(position.x,position.y))
                    .expect("failed to send");


                /* = = = = = = = = = */
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let Some(code) = input.virtual_keycode {
                    match code {
                        VirtualKeyCode::A => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Left1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::W => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Up1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::D => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Right1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::S => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Down1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Space => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Start1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::LShift => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Select1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Z | VirtualKeyCode::F => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::B1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::X | VirtualKeyCode::G => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::A1,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }


                        VirtualKeyCode::Left | VirtualKeyCode::J => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Left2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Up | VirtualKeyCode::I => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Up2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Right | VirtualKeyCode::L =>  {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Right2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Down | VirtualKeyCode::K => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Down2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Return => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Start2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::RShift => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::Select2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Numpad1 | VirtualKeyCode::Semicolon => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::B2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        VirtualKeyCode::Numpad2 | VirtualKeyCode::Apostrophe => {
                            control_tx
                                .send(Message::Button(
                                    ButtonName::A2,
                                    input.state == ElementState::Pressed,
                                ))
                                .expect("failed to send");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        *control_flow = ControlFlow::WaitUntil(Instant::now() + wait_time);

        if handle.lock().unwrap().as_ref().unwrap().is_finished() {
            handle
                .lock()
                .unwrap()
                .take()
                .expect("cpu emulation exited unexpectedly");
            return;
        }

        if Instant::now().duration_since(last) > wait_time {
            screen.redraw();
            last = Instant::now();
        }
    });
}
