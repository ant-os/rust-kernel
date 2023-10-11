type Color = u32;

use crate::bitmap_font::{BitmapCharImpl, BitmapFont};
use crate::framebuffer::Framebuffer;

pub struct Renderer {
    target_fb: &'static Framebuffer,
    foreground_color: Color,
    background_color: Color,
    bitmap_font: &'static BitmapFont,
}

pub enum RendererError {
    OutOfBounds,
}

impl Renderer {
    /*
        Creates a new [Renderer] with a given [Framebuffer] and a [BitmapFont] with default colors.
    */
    pub fn new(fb: &'static Framebuffer, font: &'static BitmapFont) -> Renderer {
        Self {
            target_fb: fb,
            foreground_color: 0xFFFFFFFF,
            background_color: 0x00000000,
            bitmap_font: font,
        }
    }

    pub unsafe fn unsafe_put_pixel(&self, x: usize, y: usize, color: Color) {
        let pixel_offset = x * (self.target_fb.pitch as usize + y);
        *(self
            .target_fb
            .address
            .as_ptr()
            .unwrap()
            .offset(pixel_offset as isize) as *mut Color) = color;
    }

    pub unsafe fn unsafe_pull_pixel(&self, x: usize, y: usize) -> Color {
        let pixel_offset = x * (self.target_fb.pitch as usize + y);
        *(self
            .target_fb
            .address
            .as_ptr()
            .unwrap()
            .offset(pixel_offset as isize) as *mut Color)
    }

    pub unsafe fn unsafe_draw_char(&self, off_x: usize, off_y: usize, chr: i8) {
        for x in 0..8 as usize {
            for y in 0..8 as usize {
                self.unsafe_put_pixel(
                    off_x + x,
                    off_y + y,
                    if self.bitmap_font[chr as usize].is_set(x, y) {
                        self.foreground_color
                    } else {
                        self.background_color
                    },
                );
            }
        }
    }
}
