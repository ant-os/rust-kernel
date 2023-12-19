pub type Color = u32;

use core::ops::Add;
use core::ptr::{addr_of, NonNull};

use crate::bitmap_font::{BitmapFont, DisplayChar};
use crate::common::endl;
use crate::tty::KERNEL_CONSOLE;
use crate::{debug, assign_uninit};
use crate::framebuffer::Framebuffer;

pub struct Renderer {
    target_fb: &'static Framebuffer,
    foreground_color: Color,
    background_color: Color,
    bitmap_font: &'static BitmapFont,
    pub optional_font_scaling: Option<u64>,
}

fn align_number_to(num: usize, align: usize) -> usize{
    num - (num % align)
}

fn mod_max(num: usize, max: usize) -> usize{
    if num > max{
        max
    }else {
        num
    }
}

pub enum RendererError {
    OutOfBounds,
}

crate::decl_uninit!{
    GLOBAL_RENDERER => Renderer
}



impl Renderer {

    pub fn global<'a>() -> &'a Renderer{
        unsafe { &*self::GLOBAL_RENDERER.as_ptr() }
    }

    pub fn global_mut<'a>() -> &'a mut Renderer{
        unsafe { &mut *self::GLOBAL_RENDERER.as_mut_ptr() }
    }

    pub fn make_global(self){
        assign_uninit!{
            GLOBAL_RENDERER (Renderer) <= self
        }
    }

    /*
        Creates a new [Renderer] with a given [Framebuffer] and a [BitmapFont] with default colors.
    */
    pub fn new(fb: &'static Framebuffer, font: &'static BitmapFont) -> Renderer {
        Self {
            target_fb: fb,
            foreground_color: 0xFFFFFFFF,
            background_color: 0x00000000,
            bitmap_font: font,
            optional_font_scaling: None,
        }
    }

    pub fn get_font_scaling(&self) -> u64{
        self.optional_font_scaling.unwrap_or(10)
    }

    

    pub unsafe fn scroll(&mut self, amount: usize, align: usize){
        let mut base = self.target_fb.address.as_ptr()
            .map(|p| core::mem::transmute::<*mut u8, *mut Color>(p))
            .unwrap();


        // 4 0
        let chunk_size = self.target_fb.width as usize * amount;
        let area_size = (self.target_fb.height * self.target_fb.width) as usize - chunk_size;
        let end_area = base.add(chunk_size);

        base.copy_from(base.add(chunk_size), area_size);

        Self::_fill_with_color(base.add(area_size), chunk_size, self.background_color, self.background_color)
    }

    pub unsafe fn clear(&self, color: Color) {
        Self::_fill_with_color(self.target_fb.address.as_ptr().map(|p|core::mem::transmute::<*mut u8, *mut Color>(p)).unwrap(), (self.target_fb.height * self.target_fb.width) as usize, color, color);
    }

    pub unsafe fn unsafe_put_pixel(&self, x: usize, y: usize, color: Color) {
        let mut pixel_offset = (mod_max(x, self.target_fb.width as usize) * 4) + self.target_fb.pitch as usize * mod_max(y, self.target_fb.height as usize);


        debug!(
            "unsafe_put_pixel( x=",
            crate::integer_to_string(x),
            ", y=",
            crate::integer_to_string(y),
            ", offset =",
            crate::integer_to_string(pixel_offset),
            " )",
            endl!()
        );

        let mut pix = core::mem::transmute::<*mut u8, *mut Color>(
            self.target_fb
                .address
                .as_ptr()
                .expect("Failed to get Pointer")
                .offset(pixel_offset as isize),
        );

        pix.write(color)
    }

    pub unsafe fn unsafe_pull_pixel(&self, x: usize, y: usize) -> Color {
        let pixel_offset = (x * 4) + (self.target_fb.pitch as usize * y);
        *(self
            .target_fb
            .address
            .as_ptr()
            .unwrap()
            .offset(pixel_offset as isize) as *mut Color)
    }

    pub fn set_text_colors_via_invert(&mut self, color: Color){
        self.foreground_color = color;
        self.background_color = !color;
    }

    pub fn update_colors(&mut self, fg_color: Option<Color>, bg_color: Option<Color>){
        self.foreground_color = fg_color.unwrap_or(self.foreground_color);
        self.background_color = bg_color.unwrap_or(self.background_color);
    }

    pub unsafe fn _fill_with_color(
        base: *mut Color,
        amount: usize,
        filler: Color,
        background_color: Color,
    ) {
        for offset in 0..amount {
            let ptr = base.offset(offset as isize);
            let val = ptr.read();
            ptr.write(filler)
        }
    }

    pub unsafe fn unsafe_put_scaled_pixel(&self, x: usize, y: usize, color: Color){
        let scaling = self.optional_font_scaling.unwrap_or(10) as usize;

        self.unsafe_fill_square(
            x * scaling,
            y * scaling,
            scaling,
            scaling,
            color
        );
    }

   pub unsafe fn unsafe_draw_line(&self, x0: usize, y0: usize, x1: usize, y1: usize, color: Color) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = -(y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 as isize } else { -1 };
        let sy = if y0 < y1 { 1 as isize } else { -1 };
        let mut err = dx + dy;
        let mut x = x0 as isize;
        let mut y = y0 as isize;
    
        while x != (x1 as isize) || y != (y1 as isize) {
            self.unsafe_put_scaled_pixel(x as usize, y as usize, color);
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    

    pub unsafe fn unsafe_fill_square(&self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for y_off in y..(y + h) {
            Self::_fill_with_color(
                self.target_fb
                    .address
                    .as_ptr()
                    .map(|p| core::mem::transmute::<*mut u8, *mut Color>(p))
                    .unwrap()
                    .offset(((x) + (self.target_fb.pitch as usize * (y_off) / 4)) as isize),
                w * 4,
                color,
                self.background_color,
            );
        }
    }

    pub fn dimensions(&self) -> (usize, usize){
        (self.target_fb.width as usize, self.target_fb.height as usize)
    }

    pub unsafe fn unsafe_draw_char(&self, off_x: usize, off_y: usize, chr: u8) -> usize {
        let scaling = self.optional_font_scaling.unwrap_or(10) as usize;
        let line_off = 0;

        for x in 0..8 as usize {
            for y in 0..8 as usize {
                self.unsafe_fill_square(
                    off_x + (x * scaling),
                    off_y + (y * scaling),
                    scaling,
                    scaling,
                    if self.bitmap_font[chr as usize].is_set(x, y) {
                        self.foreground_color
                    } else {
                        self.background_color
                    },
                );
            }
        }

        line_off
    }

    pub unsafe fn draw_raw_image(&self, x: usize, y: usize, pixels: &'_ [u8]){
        self.target_fb.address
            .as_ptr()
            .unwrap()
            .offset((x + (self.target_fb.pitch as usize * (y))) as isize)
            .copy_from(pixels.as_ptr(), pixels.len());
    }
    


    pub unsafe fn unsafe_draw_text(&self, x: usize, y: usize, text: &'_ str) -> usize{
        let scaling = self.optional_font_scaling.unwrap_or(10) as usize;
        let mut line_off = 0usize;

        for (index, chr) in text.chars().enumerate() {
            if chr == '\n'{
                line_off += 1;
                continue;
            }

            let x_off = x + (index * (16 * scaling));


            line_off += self.unsafe_draw_char(x_off, y + (line_off * 16), chr as u8);
        }

        line_off
    }
}
