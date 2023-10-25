use crate::ppu::colors::Color;
use crate::WIDTH;
use pixels::Pixels;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use winit::window::Window;

/// A struct containg all the buttons for one controller and whether they are pressed (`true`) or not (`false`)
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Buttons {
    pub a1: bool,
    pub b1: bool,
    pub up1: bool,
    pub down1: bool,
    pub left1: bool,
    pub right1: bool,
    pub select1: bool,
    pub start1: bool,

    pub a2: bool,
    pub b2: bool,
    pub up2: bool,
    pub down2: bool,
    pub left2: bool,
    pub right2: bool,
    pub select2: bool,
    pub start2: bool,
}

impl Buttons {
    pub fn get_by_index(self, idx: u8) -> bool {
        match idx {
            0 => self.a1,
            1 => self.b1,
            2 => self.select1,
            3 => self.start1,
            4 => self.up1,
            5 => self.down1,
            6 => self.left1,
            7 => self.right1,
            8 => self.a2,
            9 => self.b2,
            10 => self.select2,
            11 => self.start2,
            12 => self.up2,
            13 => self.down2,
            14 => self.left2,
            15 => self.right2,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ButtonName {
    A1,
    B1,
    Up1,
    Down1,
    Left1,
    Right1,
    Start1,
    Select1,
    A2,
    B2,
    Up2,
    Down2,
    Left2,
    Right2,
    Start2,
    Select2,
}

pub enum ScreenReader {
    Dummy,
    Real {
        pixels: Box<Mutex<Pixels>>,
        window: Window,
    },
}

pub enum Message {
    Button(ButtonName, bool),
    Pause(bool),
    PixelPointed(f64,f64),
}

#[derive(Clone)]
pub struct Screen(pub Arc<ScreenReader>);

pub enum ScreenWriter {
    Dummy,
    Real {
        screen: Screen,
        pixels: Vec<u8>,
        control_rx: Receiver<Message>,
    },
}

impl ScreenWriter {
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) {
        if let Self::Real { pixels, .. } = self {
            pixels[4 * (y * WIDTH as usize + x)] = color.0;
            pixels[4 * (y * WIDTH as usize + x) + 1] = color.1;
            pixels[4 * (y * WIDTH as usize + x) + 2] = color.2;
            pixels[4 * (y * WIDTH as usize + x) + 3] = 0xff;
        }
    }

    pub fn render_frame(&mut self) {
        if let Self::Real { pixels, screen, .. } = self {
            if let ScreenReader::Real {
                pixels: reader_pixels,
                ..
            } = &*screen.0
            {
                reader_pixels
                    .lock()
                    .expect("failed to lock")
                    .frame_mut()
                    .clone_from_slice(pixels);
            }
        }
    }
}

impl Screen {
    pub fn dummy() -> (Screen, ScreenWriter) {
        (Screen(Arc::new(ScreenReader::Dummy)), ScreenWriter::Dummy)
    }

    pub fn new(pixels: Pixels, window: Window) -> (Self, ScreenWriter, Sender<Message>) {
        let buf = pixels.frame().to_vec();
        let (tx, rx) = channel();

        let screen = Screen(Arc::new(ScreenReader::Real {
            pixels: Box::new(Mutex::new(pixels)),
            window,
        }));

        (
            screen.clone(),
            ScreenWriter::Real {
                screen,
                pixels: buf,
                control_rx: rx,
            },
            tx,
        )
    }

    pub fn redraw(&mut self) {
        if let ScreenReader::Real { pixels, .. } = &*self.0 {
            pixels
                .lock()
                .expect("failed to lock")
                .render()
                .expect("failed to render using pixels library");
        }
    }
}
