#![allow(dead_code)]
use anyhow::Result;
use glow::*;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::gettext::gettext;
use crate::ndata;
use crate::ngl::{Context, CONTEXT};
use crate::{formatx, nelog, warn};

pub enum ShaderType {
    Fragment,
    Vertex,
}
impl ShaderType {
    pub fn to_gl(&self) -> u32 {
        match self {
            ShaderType::Fragment => glow::FRAGMENT_SHADER,
            ShaderType::Vertex => glow::VERTEX_SHADER,
        }
    }
}

pub struct Shader {
    pub vertname: String,
    pub fragname: String,
    pub program: glow::Program,
}

impl Shader {
    fn compile(
        ctx: &Context,
        shadertype: ShaderType,
        name: &str,
        source: &str,
    ) -> Result<glow::Shader> {
        let gl = &ctx.gl;
        let shader = unsafe {
            gl.create_shader(shadertype.to_gl())
                .map_err(|e| anyhow::anyhow!(e))?
        };
        unsafe {
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
        }
        if unsafe { !gl.get_shader_compile_status(shader) } {
            for (i, line) in source.lines().enumerate() {
                nelog!("{:04}: {}", i, line);
            }
            let slog = unsafe { gl.get_shader_info_log(shader) };
            warn!("Failed to compile shader '{}': [[\n{}\n]]", name, slog);
            return Err(anyhow::anyhow!("failed to compile shader program"));
        }
        Ok(shader)
    }

    fn link(
        ctx: &Context,
        vertshader: glow::Shader,
        fragshader: glow::Shader,
    ) -> Result<glow::Program> {
        let gl = &ctx.gl;
        let program = unsafe { gl.create_program().map_err(|e| anyhow::anyhow!(e))? };
        unsafe {
            gl.attach_shader(program, vertshader);
            gl.attach_shader(program, fragshader);
            gl.link_program(program);
            gl.delete_shader(vertshader);
            gl.delete_shader(fragshader);
        }
        if unsafe { !gl.get_program_link_status(program) } {
            let slog = unsafe { gl.get_program_info_log(program) };
            warn!("Failed to link shader: [[\n{}\n]]", slog);
            return Err(anyhow::anyhow!("failed to link shader program"));
        }
        Ok(program)
    }
}

enum ShaderSource {
    Path(String),
    Data(String),
    None,
}
impl ShaderSource {
    const INCLUDE_INSTRUCTION: &str = "#include";
    const GLSL_PATH: &str = "glsl/";

    /// Really simple preprocessor
    fn preprocess(data: &str) -> Result<String> {
        let mut module_string = String::new();
        for line in data.lines() {
            let line = line.trim();
            if line.starts_with(Self::INCLUDE_INSTRUCTION) {
                match line.split("\"").nth(1) {
                    Some(include) => {
                        let include_string = Self::load_file(include)?;
                        module_string.push_str(&include_string);
                        module_string.push('\n');
                    }
                    None => {
                        return Err(anyhow::anyhow!("#include syntax error"));
                    }
                }
            } else {
                module_string.push_str(line);
                module_string.push('\n');
            }
        }
        Ok(module_string)
    }

    fn load_file(path: &str) -> Result<String> {
        let fullpath = format!("{}{}", Self::GLSL_PATH, path);
        let rawdata = ndata::read(&fullpath)?;
        let data = std::str::from_utf8(&rawdata)?;
        Self::preprocess(data)
    }

    pub fn to_string(&self) -> Result<String> {
        match self {
            Self::Path(path) => Self::load_file(&path),
            Self::Data(data) => Self::preprocess(&data),
            Self::None => Err(anyhow::anyhow!("no shader source defined!")),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Path(path) => path.clone(),
            Self::Data(_) => String::from("DATA"),
            Self::None => String::from("NONE"),
        }
    }
}

struct ShaderBuilder {
    name: Option<String>,
    vert: ShaderSource,
    frag: ShaderSource,
    prepend: String,
}
impl ShaderBuilder {
    pub fn new(name: Option<&str>) -> Self {
        ShaderBuilder {
            name: name.map(String::from),
            vert: ShaderSource::None,
            frag: ShaderSource::None,
            prepend: Default::default(),
        }
    }

    pub fn vert_file(mut self, path: &str) -> Self {
        self.vert = ShaderSource::Path(String::from(path));
        self
    }

    pub fn frag_file(mut self, path: &str) -> Self {
        self.frag = ShaderSource::Path(String::from(path));
        self
    }

    pub fn vert_data(mut self, data: &str) -> Self {
        self.vert = ShaderSource::Data(String::from(data));
        self
    }

    pub fn frag_data(mut self, data: &str) -> Self {
        self.frag = ShaderSource::Data(String::from(data));
        self
    }

    pub fn prepend(mut self, data: &str) -> Self {
        self.prepend = String::from(data);
        self
    }

    pub fn build(self, ctx: &Context) -> Result<Shader> {
        let mut vertdata = ShaderSource::to_string(&self.vert)?;
        let mut fragdata = ShaderSource::to_string(&self.frag)?;

        let glsl = unsafe { naevc::gl_screen.glsl };
        let mut prepend = format!("#version {}\n\n#define GLSL_VERSION {}\n", glsl, glsl);
        prepend.push_str("#define HAS_GL_ARB_shader_subroutine 1\n");

        if self.prepend.len() > 0 {
            vertdata.insert_str(0, &self.prepend);
            fragdata.insert_str(0, &self.prepend);
        }
        vertdata.insert_str(0, &prepend);
        fragdata.insert_str(0, &prepend);

        let vertname = self.vert.name();
        let fragname = self.frag.name();

        let vertshader = Shader::compile(ctx, ShaderType::Vertex, &vertname, &vertdata)?;
        let fragshader = Shader::compile(ctx, ShaderType::Fragment, &fragname, &fragdata)?;
        let program = Shader::link(ctx, vertshader, fragshader)?;

        Ok(Shader {
            vertname,
            fragname,
            program,
        })
    }
}

#[no_mangle]
pub extern "C" fn gl_program_backend(
    cvert: *const c_char,
    cfrag: *const c_char,
    cprepend: *const c_char,
) -> u32 {
    let ctx = CONTEXT.get().unwrap(); /* Lock early. */
    let vert = unsafe { CStr::from_ptr(cvert) };
    let frag = unsafe { CStr::from_ptr(cfrag) };
    let mut sb = ShaderBuilder::new(None)
        .vert_file(vert.to_str().unwrap())
        .frag_file(frag.to_str().unwrap());

    if !cprepend.is_null() {
        let prepend = unsafe { CStr::from_ptr(cprepend) };
        sb = sb.prepend(prepend.to_str().unwrap());
    }

    sb.build(&ctx).unwrap().program.0.into()
}

#[no_mangle]
pub extern "C" fn gl_program_vert_frag_string(
    cvert: *const c_char,
    vert_size: usize,
    cfrag: *const c_char,
    frag_size: usize,
) -> u32 {
    let ctx = CONTEXT.get().unwrap(); /* Lock early. */
    let vertdata =
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(cvert as *const u8, vert_size) })
            .unwrap();
    let fragdata =
        std::str::from_utf8(unsafe { std::slice::from_raw_parts(cfrag as *const u8, frag_size) })
            .unwrap();
    ShaderBuilder::new(None)
        .vert_data(vertdata)
        .frag_data(fragdata)
        .build(&ctx)
        .unwrap()
        .program
        .0
        .into()
}
