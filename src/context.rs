#![allow(dead_code)]
use anyhow::Result;
use glow::*;
use nalgebra::{Matrix3, Vector4};
use sdl2 as sdl;
use sdl2::image::ImageRWops;
use std::ops::Deref;
use std::os::raw::c_double;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::ThreadId;

use crate::buffer::{
    Buffer, BufferBuilder, BufferTarget, BufferUsage, VertexArray, VertexArrayBuffer,
    VertexArrayBuilder,
};
use crate::render::{SolidUniform, TextureUniform};
use crate::shader::{Shader, ShaderBuilder};
use crate::{debug, formatx, warn};
use crate::{gettext, log, ndata};

fn debug_callback(source: u32, msg_type: u32, id: u32, severity: u32, msg: &str) {
    let s_source = match source {
        glow::DEBUG_SOURCE_API => "api",
        glow::DEBUG_SOURCE_WINDOW_SYSTEM => "window_system",
        glow::DEBUG_SOURCE_SHADER_COMPILER => "shader_compiler",
        glow::DEBUG_SOURCE_THIRD_PARTY => "third_party",
        glow::DEBUG_SOURCE_APPLICATION => "application",
        glow::DEBUG_SOURCE_OTHER => "other",
        _ => &format!("{}", source),
    };
    let s_type = match msg_type {
        glow::DEBUG_TYPE_ERROR => "error",
        glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "deprecated_behavior",
        glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "undefined_behavior",
        glow::DEBUG_TYPE_PORTABILITY => "portability",
        glow::DEBUG_TYPE_PERFORMANCE => "performance",
        glow::DEBUG_TYPE_MARKER => "marker",
        glow::DEBUG_TYPE_PUSH_GROUP => "push_group",
        glow::DEBUG_TYPE_POP_GROUP => "pop_group",
        glow::DEBUG_TYPE_OTHER => "other",
        _ => &format!("{}", msg_type),
    };
    let s_id = format!("{}", id);
    let s_severity = match severity {
        glow::DEBUG_SEVERITY_LOW => "low",
        glow::DEBUG_SEVERITY_MEDIUM => "medium",
        glow::DEBUG_SEVERITY_HIGH => "high",
        glow::DEBUG_SEVERITY_NOTIFICATION => "notification",
        _ => &format!("{}", severity),
    };

    if severity == glow::DEBUG_SEVERITY_LOW {
        debug!(
            "OpenGL debug( source={}, type={}, id={}, severity={} ): {}",
            s_source, s_type, s_id, s_severity, msg
        );
    } else {
        warn!(
            "OpenGL debug( source={}, type={}, id={}, severity={} ): {}",
            s_source, s_type, s_id, s_severity, msg
        );
    }
}

#[allow(clippy::enum_variant_names)] // Remove when we add other messages
#[derive(Clone)]
pub enum Message {
    DeleteBuffer(glow::NativeBuffer),
    DeleteVertexArray(glow::NativeVertexArray),
    DeleteProgram(glow::NativeProgram),
    DeleteTexture(glow::NativeTexture),
    DeleteSampler(glow::NativeSampler),
    DeleteFramebuffer(glow::NativeFramebuffer),
}
impl Message {
    fn execute(self, ctx: &Context) {
        match self {
            Self::DeleteBuffer(buf) => unsafe {
                ctx.gl.delete_buffer(buf);
            },
            Self::DeleteVertexArray(vao) => unsafe {
                ctx.gl.delete_vertex_array(vao);
            },
            Self::DeleteProgram(pgm) => unsafe {
                ctx.gl.delete_program(pgm);
            },
            Self::DeleteTexture(tex) => unsafe {
                ctx.gl.delete_texture(tex);
            },
            Self::DeleteSampler(smp) => unsafe {
                ctx.gl.delete_sampler(smp);
            },
            Self::DeleteFramebuffer(buf) => unsafe {
                ctx.gl.delete_framebuffer(buf);
            },
        }
    }
}

pub static CONTEXT: OnceLock<Context> = OnceLock::new();
pub static MESSAGE_QUEUE: Mutex<Vec<Message>> = Mutex::new(vec![]);

pub struct Context {
    pub sdlvid: sdl::VideoSubsystem,
    pub gl: glow::Context,
    pub window: sdl::video::Window,
    pub gl_context: sdl::video::GLContext,
    main_thread: ThreadId,
    pub window_width: u32,  // In real pixels
    pub window_height: u32, // In real pixels
    pub view_width: f32,    // In scaled pixels
    pub view_height: f32,   // In scaled pixels
    pub view_scale: f32,    // In scaling value

    // Useful "globals"
    pub projection: Matrix3<f32>,
    pub program_texture: Shader,
    pub buffer_texture: Buffer,
    pub program_solid: Shader,
    pub buffer_solid: Buffer,
    pub vbo_square: Buffer,
    pub vao_square: VertexArray,
    pub vbo_center: Buffer,
    pub vao_center: VertexArray,
    pub vbo_triangle: Buffer,
    pub vao_triangle: VertexArray,

    // To be phased out when moved to rust
    pub vao_core: glow::VertexArray,
}
unsafe impl Send for Context {}
unsafe impl Sync for Context {}

/// Wrapper for a Context MutexGuard
pub struct ContextWrap<'sc, 'ctx>(MutexGuard<'sc, &'ctx Context>);
impl<'sc, 'ctx> ContextWrap<'sc, 'ctx> {
    fn new(guard: MutexGuard<'sc, &'ctx Context>) -> Self {
        guard.window.gl_make_current(&guard.gl_context).unwrap();
        ContextWrap(guard)
    }
}
impl<'ctx> Deref for ContextWrap<'_, 'ctx> {
    type Target = &'ctx Context;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for ContextWrap<'_, '_> {
    fn drop(&mut self) {
        self.0.sdlvid.gl_release_current_context().unwrap();
    }
}

/// Wrapper for thread safe OpenGL context
pub struct SafeContext<'ctx> {
    ctx: Arc<Mutex<&'ctx Context>>,
}
impl<'ctx> SafeContext<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        ctx.sdlvid.gl_release_current_context().unwrap();
        SafeContext {
            ctx: Arc::new(Mutex::new(ctx)),
        }
    }
    pub fn lock(&self) -> ContextWrap<'_, 'ctx> {
        let guard = self.ctx.lock().unwrap();
        ContextWrap::new(guard)
    }
}
impl Drop for SafeContext<'_> {
    fn drop(&mut self) {
        let guard = self.ctx.lock().unwrap();
        guard.window.gl_make_current(&guard.gl_context).unwrap();
    }
}

impl Context {
    #[rustfmt::skip]
    const DATA_SQUARE: [f32;8] = [ 0., 0.,
                                   1., 0.,
                                   0., 1.,
                                   1., 1., ];
    #[rustfmt::skip]
    const DATA_CENTER: [f32;8] = [-1.,-1.,
                                   1.,-1.,
                                  -1., 1.,
                                   1., 1., ];

    // cos/sin are not constant or we would want
    //vertex[0]      = 0.5 * cos( 4. * M_PI / 3. );
    //vertex[1]      = 0.5 * sin( 4. * M_PI / 3. );
    //vertex[2]      = 0.5 * cos( 0. );
    //vertex[3]      = 0.5 * sin( 0. );
    //vertex[4]      = 0.5 * cos( 2. * M_PI / 3. );
    //vertex[5]      = 0.5 * sin( 2. * M_PI / 3. );
    //vertex[6]      = vertex[0];
    //vertex[7]      = vertex[1];
    #[rustfmt::skip]
    const DATA_TRIANGLE: [f32;8] = [
        -0.25, -0.433_012_7,
         0.5,   0.0,
        -0.25,  0.433_012_7,
        -0.25, -0.433_012_7];

    fn create_context(
        sdlvid: &sdl::VideoSubsystem,
        gl_attr: &sdl::video::gl_attr::GLAttr,
        major: u8,
        minor: u8,
    ) -> Result<(sdl::video::Window, sdl::video::GLContext)> {
        let (width, height, resizable, borderless) = unsafe {
            (
                naevc::conf.width,
                naevc::conf.height,
                naevc::conf.notresizable == 0,
                naevc::conf.borderless != 0,
            )
        };
        gl_attr.set_context_version(major, minor);

        //() = crate::start::start().name.into_string()?.as_str();

        let mut wdwbuild = sdlvid.window(
            format!(
                "{} - {}",
                crate::APPNAME,
                crate::start::start().name.clone().into_string()?
            )
            .as_str(),
            width.max(naevc::RESOLUTION_W_MIN),
            height.max(naevc::RESOLUTION_H_MIN),
        );
        if resizable {
            wdwbuild.resizable();
        }
        if borderless {
            wdwbuild.borderless();
        }
        wdwbuild.opengl().position_centered().allow_highdpi();
        let mut window = wdwbuild.build()?;

        window
            .set_minimum_size(naevc::RESOLUTION_W_MIN, naevc::RESOLUTION_H_MIN)
            .unwrap_or_else(|_| log::warn("Unable to set minimum window size."));
        let gl_context = match window.gl_create_context() {
            Ok(ctx) => ctx,
            Err(e) => anyhow::bail!("Unable to create OpenGL context: {}", e),
        };

        // Try to load the icon.
        let filename = format!("{}{}", ndata::GFX_PATH, "icon.webp");
        match ndata::rwops(filename.as_str()) {
            Ok(rw) => match rw.load() {
                Ok(icon) => window.set_icon(icon),
                Err(e) => anyhow::bail!(e),
            },
            Err(e) => anyhow::bail!(e),
        }

        Ok((window, gl_context))
    }

    pub fn new(sdlvid: sdl::VideoSubsystem) -> Result<&'static Self> {
        let (minimize, fsaa, vsync) = unsafe {
            (
                naevc::conf.minimize != 0,
                naevc::conf.fsaa,
                naevc::conf.vsync != 0,
            )
        };

        /* Focus behaviour. */
        sdl::hint::set_video_minimize_on_focus_loss(minimize);

        /* Set up the attributes. */
        let gl_attr = sdlvid.gl_attr();
        gl_attr.set_context_profile(sdl::video::GLProfile::Core);
        gl_attr.set_double_buffer(true);
        if fsaa > 1 {
            gl_attr.set_multisample_buffers(1);
            gl_attr.set_multisample_samples(fsaa);
        }
        gl_attr.set_framebuffer_srgb_compatible(true);
        gl_attr.set_context_flags().forward_compatible().set();

        #[cfg(debug_assertions)]
        gl_attr.set_context_flags().debug().set();

        let (window, gl_context) = match Self::create_context(&sdlvid, &gl_attr, 4, 6) {
            Ok(v) => v,
            _ => match Self::create_context(&sdlvid, &gl_attr, 3, 3) {
                Ok(v) => v,
                _ => anyhow::bail!("Foo"),
            },
        };
        let mut gl = unsafe {
            glow::Context::from_loader_function(|s| sdlvid.gl_get_proc_address(s) as *const _)
        };

        // Final touches
        sdlvid
            .gl_set_swap_interval(match vsync {
                true => 1,
                false => 0,
            })
            .unwrap_or_else(|_| log::warn("Unable to set OpenGL swap interval!"));

        match gl_attr.framebuffer_srgb_compatible() {
            true => (),
            false => log::warn("Unable to set framebuffer to SRGB!"),
        };

        #[cfg(debug_assertions)]
        match gl_attr.context_flags().has_debug() {
            true => unsafe {
                gl.enable(glow::DEBUG_OUTPUT);
                gl.debug_message_callback(debug_callback);
                // Hide stuff that is useful for RenderDoc
                for msg_type in [
                    glow::DEBUG_TYPE_PUSH_GROUP,
                    glow::DEBUG_TYPE_POP_GROUP,
                    glow::DEBUG_TYPE_MARKER,
                ] {
                    gl.debug_message_control(
                        glow::DEBUG_SOURCE_APPLICATION,
                        msg_type,
                        glow::DONT_CARE,
                        &[],
                        false,
                    );
                }
                // Notifications about putting stuff in GPU memory, which we want
                gl.debug_message_control(
                    glow::DONT_CARE,
                    glow::DONT_CARE,
                    glow::DEBUG_SEVERITY_NOTIFICATION,
                    &[],
                    false,
                );
                // NVIDIA warnings about recompiling shaders, don't think we can do much here
                gl.debug_message_control(
                    glow::DEBUG_SOURCE_API,
                    glow::DEBUG_TYPE_PERFORMANCE,
                    glow::DONT_CARE,
                    &[131218],
                    false,
                );
            },
            false => log::warn("Unable to set OpenGL debug mode!"),
        };

        unsafe {
            naevc::gl_screen.window = window.raw() as *mut naevc::SDL_Window;
            naevc::gl_screen.context = gl_context.raw();
            (naevc::gl_screen.major, naevc::gl_screen.minor) = gl_attr.context_version();
            let major: i32 = naevc::gl_screen.major.into();
            let minor: i32 = naevc::gl_screen.minor.into();
            naevc::gl_screen.glsl = if major * 100 + minor * 10 > 320 {
                100 * major + 10 * minor
            } else {
                150
            };
            naevc::gl_screen.depth = gl_attr.depth_size();
            naevc::gl_screen.r = gl_attr.red_size();
            naevc::gl_screen.g = gl_attr.green_size();
            naevc::gl_screen.b = gl_attr.blue_size();
            naevc::gl_screen.a = gl_attr.alpha_size();
            naevc::gl_screen.depth =
                naevc::gl_screen.r + naevc::gl_screen.g + naevc::gl_screen.b + naevc::gl_screen.a;
            naevc::gl_screen.fsaa = gl_attr.multisample_samples();
            naevc::gl_screen.tex_max = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            naevc::gl_screen.multitex_max = gl.get_parameter_i32(glow::MAX_TEXTURE_IMAGE_UNITS);
        }

        // Modern OpenGL requires at least one VAO and the C code uses the same one
        let vao_core = unsafe {
            let vao = gl.create_vertex_array().map_err(|e| anyhow::anyhow!(e))?;
            gl.bind_vertex_array(Some(vao));
            gl.object_label(
                glow::VERTEX_ARRAY,
                vao.0.into(),
                Some("C Core Vertex Array"),
            );
            vao
        };

        // Initialize some useful globals
        // The texture shader
        let program_texture = ShaderBuilder::new(Some("Texture Shader"))
            .vert_file("rust_texture.vert")
            .frag_file("rust_texture.frag")
            .sampler("sampler", 0)
            .build(&gl)?;
        let uniform = TextureUniform::default();
        let buffer_texture = BufferBuilder::new(Some("Texture Buffer"))
            .target(BufferTarget::Uniform)
            .usage(BufferUsage::Dynamic)
            .data(&uniform.buffer()?)
            .build(&gl)?;
        // The solid shader
        let program_solid = ShaderBuilder::new(Some("Solid Shader"))
            .vert_file("rust_solid.vert")
            .frag_file("rust_solid.frag")
            .build(&gl)?;
        let uniform = SolidUniform::default();
        let buffer_solid = BufferBuilder::new(Some("Solid Buffer"))
            .target(BufferTarget::Uniform)
            .usage(BufferUsage::Dynamic)
            .data(&uniform.buffer()?)
            .build(&gl)?;

        // Square VBO
        let vbo_square = BufferBuilder::new(Some("Square VBO"))
            .usage(BufferUsage::Static)
            .data_f32(&Self::DATA_SQUARE)
            .build(&gl)?;
        let vao_square = VertexArrayBuilder::new(Some("Square Vertex Array"))
            .buffers(&[VertexArrayBuffer {
                buffer: &vbo_square,
                size: 2,
                stride: 0, // tightly packed
                offset: 0,
                divisor: 0,
            }])
            .build_gl(&gl)?;

        // Center VBO
        let vbo_center = BufferBuilder::new(Some("Center VBO"))
            .usage(BufferUsage::Static)
            .data_f32(&Self::DATA_CENTER)
            .build(&gl)?;
        let vao_center = VertexArrayBuilder::new(Some("Center Vertex Array"))
            .buffers(&[VertexArrayBuffer {
                buffer: &vbo_center,
                size: 2,
                stride: 0, // tightly packed
                offset: 0,
                divisor: 0,
            }])
            .build_gl(&gl)?;

        // Triangle VBO
        let vbo_triangle = BufferBuilder::new(Some("Triangle VBO"))
            .usage(BufferUsage::Static)
            .data_f32(&Self::DATA_TRIANGLE)
            .build(&gl)?;
        let vao_triangle = VertexArrayBuilder::new(Some("Triangle Vertex Array"))
            .buffers(&[VertexArrayBuffer {
                buffer: &vbo_triangle,
                size: 2,
                stride: 0, // tightly packed
                offset: 0,
                divisor: 0,
            }])
            .build_gl(&gl)?;

        let (window_width, window_height) = window.size();
        let (view_width, view_height, view_scale) = {
            let (w, h) = (window_width as f32, window_height as f32);
            let scale = unsafe { naevc::conf.scalefactor as f32 };
            (w / scale, h / scale, 1.0 / scale)
        };
        #[rustfmt::skip]
        let projection = Matrix3::new(
            2.0/view_width, 0.0,             -1.0,
            0.0,            2.0/view_height, -1.0,
            0.0,            0.0,              1.0,
        );
        unsafe {
            gl.bind_vertex_array(Some(vao_core)); // Set default C VAO
        }
        let ctx = Context {
            sdlvid,
            window,
            gl_context,
            gl,
            main_thread: std::thread::current().id(),
            window_width,
            window_height,
            view_width,
            view_height,
            view_scale,
            projection,
            program_texture,
            buffer_texture,
            program_solid,
            buffer_solid,
            vbo_square,
            vao_square,
            vbo_center,
            vao_center,
            vbo_triangle,
            vao_triangle,
            vao_core,
        };
        let _ = CONTEXT.set(ctx);
        Ok(CONTEXT.get().unwrap())
    }

    fn is_main_thread(&self) -> bool {
        self.main_thread == std::thread::current().id()
    }

    pub fn execute_messages(&self) {
        let mut queue = MESSAGE_QUEUE.lock().unwrap();
        for msg in queue.drain(..) {
            msg.execute(self);
        }
    }

    pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, colour: Vector4<f32>) -> Result<()> {
        #[rustfmt::skip]
        let transform: Matrix3<f32> = self.projection * Matrix3::new(
             w,  0.0,  x,
            0.0,  h,   y,
            0.0, 0.0, 1.0,
        );
        let uniform = SolidUniform { transform, colour };
        self.draw_rect_ex(&uniform)
    }

    pub fn draw_rect_ex(&self, uniform: &SolidUniform) -> Result<()> {
        let gl = &self.gl;
        self.program_solid.use_program(gl);
        self.vao_square.bind(self);

        self.buffer_solid.write(self, &uniform.buffer()?)?;
        self.buffer_solid.bind_base(self, 0);
        unsafe {
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        VertexArray::unbind(self);
        self.buffer_solid.unbind(self);

        Ok(())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gl_renderRect(
    x: c_double,
    y: c_double,
    w: c_double,
    h: c_double,
    c: *mut Vector4<f32>,
) {
    let ctx = CONTEXT.get().unwrap();
    let colour = unsafe { *c };
    let _ = ctx.draw_rect(x as f32, y as f32, w as f32, h as f32, colour);
}
